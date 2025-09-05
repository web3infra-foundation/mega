#!/usr/bin/env python3

import os
import subprocess
import shutil
import json
import hashlib
import tarfile
from datetime import datetime, timezone
import uuid
from pathlib import Path
from git import Repo, Actor
from kafka import KafkaProducer
from packaging.version import parse as vparse

from .database import Base,engine
from .do_utils import load_processed_from_db, update_repo_sync_result
from .kafka_message_model import CrateTypeEnum, MessageKindEnum, MessageModel, RepoSyncModel, SourceOfDataEnum
from .model import SyncStatusEnum
import atexit
import threading

import concurrent.futures

# 并发下载配置
ENABLE_CONCURRENCY = os.environ.get("ENABLE_CONCURRENCY", "false").lower() == "true"
MAX_WORKERS = int(os.environ.get("MAX_WORKERS", "8"))


# 仓库和目录
REPO_URL = os.environ.get(
    "REPO_URL",
    "https://github.com/rust-lang/crates.io-index.git"
)
CLONE_DIR = os.environ.get("CLONE_DIR", "/opt/data/crates.io-index")
DOWNLOAD_DIR = os.environ.get("DOWNLOAD_DIR", "/opt/data/crates")

# Mega配置
MEGA_URL = os.environ.get("MEGA_URL", "http://git.gitmega.nju:30080")

# Kafka 配置
KAFKA_BROKER = os.environ.get("KAFKA_BROKER", "172.17.0.1:30092")
KAFKA_TOPIC = os.environ.get("KAFKA_TOPIC", "ANALYSIS_TEST")
KAFKA_TOPIC_INDEX = os.environ.get("KAFKA_TOPIC_INDEX", "INDEX_TEST")

MAX_RETRIES = int(os.environ.get("MAX_RETRIES", "3"))

def clone_or_update_index():
    """
    克隆或更新 crates.io-index 仓库
    
    Returns:
        bool: 操作是否成功
    """
    if not shutil.which("git"):
        print("错误: git未安装。请先安装git再运行此脚本。")
        return False

    clone_dir = os.path.abspath(CLONE_DIR)

    if os.path.exists(clone_dir):
        if os.path.isdir(os.path.join(clone_dir, '.git')):
            print(f"目录 {clone_dir} 已存在且是git仓库，尝试更新...")
            try:
                original_dir = os.getcwd()
                os.chdir(clone_dir)
                subprocess.run(["git", "pull"], check=True)
                os.chdir(original_dir)
                print("仓库更新成功。")
                return True
            except subprocess.CalledProcessError as e:
                print(f"更新失败: {e}")
                return False
        else:
            print(f"错误: 目录 {clone_dir} 已存在但不是git仓库。")
            return False
    
    print(f"正在克隆到 {clone_dir}...")
    try:
        subprocess.run(["git", "clone", REPO_URL, clone_dir], check=True)
        print(f"成功克隆到 '{clone_dir}' 目录。")
        return True
    except subprocess.CalledProcessError as e:
        print(f"克隆失败: {e}")
        return False


def get_all_files(index_dir):
    """遍历目录，返回所有crate文件信息"""
    crates = {}
    print(f"正在扫描目录：{index_dir}")
    
    try:
        for root, dirs, files in os.walk(index_dir):
            if Path(root) == Path(index_dir):
                continue
            
            for file in files:
                file_path = Path(os.path.join(root, file))
                if not file_path.is_file():
                    continue

                print(f"处理文件: {file_path}")
                
                try:
                    with open(file_path, "r", encoding="utf-8") as f:
                        lines = f.readlines()
                        for line in lines:
                            try:
                                data = json.loads(line)
                                crate_name = data["name"]
                                version = data["vers"]
                                checksum = data["cksum"]
                                
                                if crate_name not in crates:
                                    crates[crate_name] = []
                                
                                crates[crate_name].append({
                                    "version": version,
                                    "description": data.get("desc", ""),
                                    "checksum": checksum
                                })
                                
                            except json.JSONDecodeError as e:
                                print(f"跳过行（JSON错误）：{e}")
                                continue
                            except KeyError as e:
                                print(f"跳过行（缺少字段）：{e}")
                                continue
                except Exception as e:
                    print(f"跳过文件 {file_path}：读取失败，原因：{e}")
                    continue

    except Exception as e:
        print(f"遍历目录失败：{e}")
        
    return crates


def calculate_sha256(file_path):
    """
    计算文件的 SHA-256 校验和
    
    Args:
        file_path (str): 文件路径
    
    Returns:
        str: SHA-256 校验和
    """
    sha256_hash = hashlib.sha256()
    with open(file_path, "rb") as f:
        for byte_block in iter(lambda: f.read(4096), b""):
            sha256_hash.update(byte_block)
    return sha256_hash.hexdigest()


def download_crate(crate_name, version, checksum):
    """
    下载指定的 crate 包并验证校验和
    返回 (success, crate_name, version, checksum)
    """
    os.makedirs(DOWNLOAD_DIR, exist_ok=True)
    crate_dir = os.path.join(DOWNLOAD_DIR, crate_name)
    os.makedirs(crate_dir, exist_ok=True)

    url = f"https://static.crates.io/crates/{crate_name}/{crate_name}-{version}.crate"
    save_path = os.path.join(crate_dir, f"{crate_name}-{version}.crate")

    if os.path.exists(save_path):
        actual_checksum = calculate_sha256(save_path)
        if actual_checksum == checksum:
            print(f"文件已存在且校验和正确，跳过下载：{crate_name}-{version}.crate")
            return True, crate_name, version, checksum
        else:
            print(f"文件已存在但校验和不匹配，将重新下载：{crate_name}-{version}.crate")
            os.remove(save_path)

    for attempt in range(MAX_RETRIES):
        try:
            subprocess.run(
                ["wget", "-O", save_path, url],
                check=True
            )

            actual_checksum = calculate_sha256(save_path)
            if actual_checksum == checksum:
                print(f"成功下载并验证：{crate_name}-{version}.crate")
                return True, crate_name, version, checksum
            else:
                print(f"校验和不匹配，尝试重新下载 (尝试 {attempt + 1}/{MAX_RETRIES})")
                os.remove(save_path)
                continue

        except subprocess.CalledProcessError as e:
            print(f"下载失败 {crate_name}-{version}.crate: {e}")
            if attempt < MAX_RETRIES - 1:
                print(f"尝试重新下载 (尝试 {attempt + 1}/{MAX_RETRIES})")
                continue
            return False, crate_name, version, checksum

    print(f"达到最大重试次数，下载失败：{crate_name}-{version}.crate")
    return False, crate_name, version, checksum



def decompress_crate(crate_path, extract_to=None):
    """解压 .crate 文件到指定目录"""
    if extract_to is None:
        extract_to = os.getcwd()
    if not os.path.exists(extract_to):
        os.makedirs(extract_to)
    
    with tarfile.open(crate_path, "r:gz") as tar:
        tar.extractall(path=extract_to)


def init_or_clean_repo(repo_dir):
    """初始化或清空git仓库"""
    if not os.path.exists(repo_dir):
        os.makedirs(repo_dir)
        subprocess.run(["git", "init"], cwd=repo_dir, check=True)
    
    for item in os.listdir(repo_dir):
        if item == ".git":
            continue
        item_path = os.path.join(repo_dir, item)
        if os.path.isfile(item_path):
            os.remove(item_path)
        elif os.path.isdir(item_path):
            shutil.rmtree(item_path)


def copy_files(src_dir, dst_dir):
    """拷贝所有文件到目标目录"""
    for item in os.listdir(src_dir):
        s = os.path.join(src_dir, item)
        d = os.path.join(dst_dir, item)
        if os.path.isdir(s):
            shutil.copytree(s, d, dirs_exist_ok=True)
        else:
            shutil.copy2(s, d)


# def add_and_commit(repo_path, version):
#     """添加文件并提交到git仓库"""
#     os.makedirs(repo_path, exist_ok=True)

#     if not os.path.exists(os.path.join(repo_path, '.git')):
#         repo = Repo.init(repo_path)
#     else:
#         repo = Repo(repo_path)

#     author = Actor("Mega", "admin@mega.com")
#     committer = Actor("Mega", "admin@mega.com")

#     repo.index.add(["."])

#     # 安全判断是否已有 commit
#     try:
#         head_commit = repo.head.commit
#         has_commit = True
#     except ValueError:
#         has_commit = False  # 空仓库

#     if has_commit:
#         # 普通提交
#         repo.index.commit(
#             f"Commit Version: {version}",
#             author=author,
#             committer=committer
#         )
#     else:
#         # 初始提交，明确没有父提交
#         repo.index.commit(
#             f"Initial Commit Version: {version}",
#             author=author,
#             committer=committer,
#             parent_commits=[]
#         )

#     if version not in [tag.name for tag in repo.tags]:
#         repo.create_tag(version)
#     else:
#         print(f"Tag '{version}' already exists.")

#     print(f"Committed and tagged version {version} in {repo_path}")

def add_and_commit(repo_path, version, author_name="Mega", author_email="admin@mega.com"):
    """使用 subprocess 添加文件并提交到 git 仓库，并创建 tag"""
    os.makedirs(repo_path, exist_ok=True)

    # 初始化仓库（如果不存在 .git）
    if not os.path.exists(os.path.join(repo_path, '.git')):
        subprocess.run(["git", "init"], cwd=repo_path, check=True)

    # 设置安全目录，防止 dubious ownership
    subprocess.run(["git", "config", "--global", "--add", "safe.directory", repo_path], check=False)

    # 设置作者 identity（全局或仓库级均可）
    subprocess.run(["git", "config", "user.name", author_name], cwd=repo_path, check=True)
    subprocess.run(["git", "config", "user.email", author_email], cwd=repo_path, check=True)

    # 添加所有文件
    subprocess.run(["git", "add", "."], cwd=repo_path, check=True)

    # 判断是否已有 commit
    result = subprocess.run(
        ["git", "rev-parse", "--verify", "HEAD"],
        cwd=repo_path,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    has_commit = result.returncode == 0

    # 提交
    commit_message = f"Commit Version: {version}" if has_commit else f"Initial Commit Version: {version}"
    subprocess.run(
        ["git", "-c", f"user.name={author_name}", "-c", f"user.email={author_email}",
         "commit", "-m", commit_message],
        cwd=repo_path,
        check=True
    )

    # 创建 tag（如果不存在）
    tags_result = subprocess.run(
        ["git", "tag"],
        cwd=repo_path,
        stdout=subprocess.PIPE,
        text=True,
        check=True
    )
    existing_tags = tags_result.stdout.splitlines()
    if version not in existing_tags:
        subprocess.run(["git", "tag", version], cwd=repo_path, check=True)
    else:
        print(f"Tag '{version}' already exists.")

    print(f"Committed and tagged version {version} in {repo_path}")

def git_push(repo_dir, mega_url, branch="master"):
    """设置远程并推送代码"""
    subprocess.run(["git", "remote", "remove", "nju"], cwd=repo_dir, check=False)
    print(f"git remote add nju {mega_url}")
    subprocess.run(["git", "remote", "add", "nju", mega_url], cwd=repo_dir, check=True)
    
    print(f"git push -f --set-upstream nju {branch}")
    try:
        subprocess.run(["git", "push", "-f", "--set-upstream", "nju", branch], 
                      cwd=repo_dir, check=False)
    except subprocess.CalledProcessError as e:
        print(f"错误输出: {e.stderr}")
    
    try:
        subprocess.run(["git", "push", "-f" , "nju", "--tags"], cwd=repo_dir, check=False)
    except subprocess.CalledProcessError:
        print("Some tags already exist in remote, continuing...")



def remove_extension(path):
    """去掉文件扩展名"""
    return os.path.splitext(path)[0]

def download_all_crates():
    """下载所有crate包"""
    crates = get_all_files(CLONE_DIR)
    print(f"找到 {len(crates)} 个 crate。")
    
    downloaded = {}
    tasks = []

    if ENABLE_CONCURRENCY:
        print(f"启用并发下载，线程数={MAX_WORKERS}")
        with concurrent.futures.ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
            for name, versions in crates.items():
                for ver_info in versions:
                    version = ver_info["version"]
                    checksum = ver_info["checksum"]
                    tasks.append(
                        executor.submit(download_crate, name, version, checksum)
                    )
            # 收集结果
            for future in concurrent.futures.as_completed(tasks):
                try:
                    success, name, version, checksum = future.result()
                    if success:
                        downloaded.setdefault(name, []).append((version, checksum))
                except Exception as e:
                    print(f"任务执行异常: {e}")
    else:
        print("按顺序下载（未启用并发）")
        for name, versions in crates.items():
            for ver_info in versions:
                version = ver_info["version"]
                checksum = ver_info["checksum"]
                success = download_crate(name, version, checksum)
                if success:
                    downloaded.setdefault(name, []).append((version, checksum))

    # 后续逻辑不变
    processed = load_processed_from_db()
    for name, version_checksum_list in downloaded.items():
        version_checksum_list.sort(key=lambda x: vparse(x[0]))
        
        processed_entry = processed.get(name)
        latest_version = processed_entry["latest_version"] if processed_entry else None
        repo_id = processed_entry["id"] if processed_entry else None

        if latest_version:
            new_versions = [
                (v, cksum) for v, cksum in version_checksum_list 
                if vparse(v) > vparse(latest_version)
            ]
        else:
            new_versions = version_checksum_list
        
        if not new_versions:
            print(f"{name} crate 未更新，  (最新版本: {latest_version})")
            continue
        
        print(f"处理并上传 crate {name} 的 {len(new_versions)} 个新版本")
        for v, cksum in new_versions:
            process_and_upload(
                crate_name=name,
                version=v,
                checksum=cksum,
                mega_url=MEGA_URL,
                kafka_broker=KAFKA_BROKER,
                kafka_topic=KAFKA_TOPIC,
                repo_id=repo_id,
            )

# def download_all_crates():
#     """下载所有crate包"""
#     crates = get_all_files(CLONE_DIR)
#     print(f"找到 {len(crates)} 个 crate。")
    
#     downloaded = {}
#     for name, versions in crates.items():
#         for ver_info in versions:
#             version = ver_info["version"]
#             checksum = ver_info["checksum"]
#             success = download_crate(name, version, checksum)
#             if success:
#                 downloaded.setdefault(name, []).append((version, checksum))

#     processed = load_processed_from_db()
#     for name, version_checksum_list in downloaded.items():
#         version_checksum_list.sort(key=lambda x: vparse(x[0]))
        
#         processed_entry = processed.get(name)
#         latest_version = processed_entry["latest_version"] if processed_entry else None
#         repo_id = processed_entry["id"] if processed_entry else None
#         print(f"latest_version: {latest_version}")
#         print(f"repo_id: {repo_id}")

#         if latest_version:
#             new_versions = [
#                 (v, cksum) for v, cksum in version_checksum_list 
#                 if vparse(v) > vparse(latest_version)
#             ]
#         else:
#             new_versions = version_checksum_list
        
#         if not new_versions:
#             print(f"{name} crate 未更新，  (最新版本: {latest_version})")
#             continue
        
#         print(f"处理并上传 crate {name} 的 {len(new_versions)} 个新版本")
#         for v, cksum in new_versions:
#             process_and_upload(
#                 crate_name=name,
#                 version=v,
#                 checksum=cksum,
#                 mega_url=MEGA_URL,
#                 kafka_broker=KAFKA_BROKER,
#                 kafka_topic=KAFKA_TOPIC,
#                 repo_id=repo_id,
#             )
    

def send_kafka_message(topic, message_dict: dict):
    """发送 dict，自动转 JSON"""
    p = init_kafka_producer()
    try:
        p.send(topic, message_dict)
    except Exception as e:
        print(f"[Kafka] Failed to send to {topic}: {e}")

# def send_kafka_message_raw(topic, message_str: str):
#     """发送原始 JSON 字符串"""
#     p = init_kafka_producer()
#     try:
#         # 注意这里要禁用二次 json.dumps，因为 producer 已经有 value_serializer
#         p.send(topic, json.loads(message_str))
#     except Exception as e:
#         print(f"[Kafka] Failed to send raw to {topic}: {e}")

def process_and_upload(crate_name, version, checksum, mega_url, kafka_broker, kafka_topic, repo_id):
    crate_file = os.path.join(DOWNLOAD_DIR, crate_name, f"{crate_name}-{version}.crate")
    repo_dir = os.path.join(DOWNLOAD_DIR, crate_name, crate_name)
    crate_entry = os.path.join(DOWNLOAD_DIR, crate_name)

    init_or_clean_repo(repo_dir)
    decompress_crate(crate_file, crate_entry)
    
    uncompress_path = remove_extension(crate_file)
    copy_files(uncompress_path, repo_dir)
    shutil.rmtree(uncompress_path)

    add_and_commit(repo_dir, version)

    mega_url_crate = f"{mega_url}/third-party/crates/{crate_name}"
    print(f"git push to {mega_url_crate}")
    try:
        git_push(repo_dir, mega_url_crate)
        push_status = SyncStatusEnum.SUCCEED
        err_msg = None
    except Exception as e:
        print(f"Push failed: {e}")
        push_status = SyncStatusEnum.FAILED
        err_msg = str(e)

    # 更新数据库
    update_repo_sync_result(
        crate_name=crate_name,
        version=version,
        mega_url=mega_url_crate,
        status=push_status,
        err_message=err_msg
    )

    # ----------------------------
    # 构造 Kafka 消息
    # ----------------------------
    db_model = RepoSyncModel(
        id=repo_id or 0,
        crate_name=crate_name,
        github_url=None,
        mega_url=mega_url_crate,
        crate_type=CrateTypeEnum.LIB,
        status=push_status,
        err_message=err_msg
    )

    message = MessageModel(
        db_model=db_model,
        message_kind=MessageKindEnum.MEGA,
        source_of_data=SourceOfDataEnum.CRATESIO,
        timestamp=datetime.utcnow().isoformat()  + "Z",
        extra_field=None
    )

    send_kafka_message(kafka_topic, message.model_dump())

    index_message = {
        "crate_name": crate_name,
        "crate_version": version,
        "cksum": checksum,
        "data_source": "Cratesio",
        "timestamp": datetime.utcnow().isoformat()  + "Z" ,
        "version": "",
        "uuid": str(uuid.uuid4())
    }
    send_kafka_message(KAFKA_TOPIC_INDEX, index_message)



# 初始化数据库表（确保第一次运行能建表）
Base.metadata.create_all(bind=engine)

producer = None
producer_lock = threading.Lock()
flush_interval = 5

def _periodic_flush():
    """后台线程定期 flush KafkaProducer"""
    global producer
    while True:
        time.sleep(flush_interval)
        try:
            if producer is not None:
                producer.flush()
                # print("Kafka producer flushed")
        except Exception as e:
            print(f"[Kafka] Flush failed: {e}")

def init_kafka_producer():
    """初始化全局 Kafka producer（线程安全）"""
    global producer
    if producer is None:  
        with producer_lock:
            if producer is None:  # double-checked locking
                producer = KafkaProducer(
                    bootstrap_servers=[KAFKA_BROKER],
                    value_serializer=lambda v: json.dumps(v).encode("utf-8"),
                    retries=5,
                    linger_ms=20,       # 延迟 20ms 批量发送
                    batch_size=64*1024  # 批量大小 64KB
                )

                # 启动后台 flush 线程
                t = threading.Thread(target=_periodic_flush, daemon=True)
                t.start()
          
                atexit.register(lambda: producer.flush())
                atexit.register(lambda: producer.close())
    return producer

def main():
    """主函数，协调整个下载过程"""
    if clone_or_update_index():
        download_all_crates()
    else:
        print("初始化失败，退出程序")
        return 1
    return 0


if __name__ == "__main__":
    exit(main())