#!/usr/bin/env python3
import os
import subprocess
import shutil
import json
from pathlib import Path
import urllib.request
import urllib.error
import hashlib
import tarfile
from packaging.version import parse as vparse
from git import Repo, Actor
import os
from kafka import KafkaProducer
import datetime
import uuid

REPO_URL = "https://github.com/rust-lang/crates.io-index.git"
CLONE_DIR = "/opt/data/crates.io-index"
DOWNLOAD_DIR = "/opt/data/crates"
MAX_RETRIES = 3  # 最大重试次数
PROCESSED_FILE = "/opt/data/processed.json"
MEGA_URL = "http://mono-engine:8000"  # 替换为实际mega仓库地址
KAFKA_BROKER = "kafka:9092"           # 替换为实际kafka broker
KAFKA_TOPIC = "REPO_SYNC_STATUS.dev.0902"                  # 替换为实际kafka topic

def clone_or_update_index():
    """
    克隆或更新 crates.io-index 仓库
    
    Returns:
        bool: 操作是否成功
    """
    # 检查是否安装了git
    if not shutil.which("git"):
        print("错误: git未安装。请先安装git再运行此脚本。")
        return False

    # 确保路径是绝对路径
    clone_dir = os.path.abspath(CLONE_DIR)

    # 检查目录是否已存在
    if os.path.exists(clone_dir):
        if os.path.isdir(os.path.join(clone_dir, '.git')):
            print(f"目录 {clone_dir} 已存在且是git仓库，尝试更新...")
            try:
                # 进入目录并执行git pull
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
    
    # 如果目录不存在，则克隆仓库
    print(f"正在克隆到 {clone_dir}...")
    try:
        subprocess.run(["git", "clone", REPO_URL, clone_dir], check=True)
        print(f"成功克隆到 '{clone_dir}' 目录。")
        return True
    except subprocess.CalledProcessError as e:
        print(f"克隆失败: {e}")
        return False

def load_processed():
    if os.path.exists(PROCESSED_FILE):
        with open(PROCESSED_FILE, "r") as f:
            return json.load(f)  # 返回dict
    return {}

def save_processed(processed_dict):
    with open(PROCESSED_FILE, "w") as f:
        json.dump(processed_dict, f)

def download_all_crates():
    crates = get_all_files("/opt/data/tmp")
    print(f"找到 {len(crates)} 个 crate。")
    downloaded = {}
    for name, versions in crates.items():
        for ver_info in versions:
            version = ver_info["version"]
            checksum = ver_info["checksum"]  # 获取 checksum
            success = download_crate(name, version, checksum)
            if success:
                downloaded.setdefault(name, []).append((version, checksum))  # 存储 (version, checksum) 元组

    processed = load_processed()
    for name, version_checksum_list in downloaded.items():
        # 按版本号排序（使用 vparse 解析版本）
        version_checksum_list.sort(key=lambda x: vparse(x[0]))
        
        last_processed = processed.get(name)
        # 只处理大于 last_processed 的版本
        if last_processed:
            new_versions = [
                (v, cksum) for v, cksum in version_checksum_list 
                if vparse(v) > vparse(last_processed)
            ]
        else:
            new_versions = version_checksum_list
        
        if not new_versions:
            print(f"跳过已处理: {name} (最新: {last_processed})")
            continue
        
        # 更新 processed 记录最新版本
        processed[name] = new_versions[-1][0]  # 只存版本号
        
        # 处理并上传每个新版本（传入 checksum）
        print(f"处理并上传 {name} 的 {len(new_versions)} 个新版本")
        for v, cksum in new_versions:
            process_and_upload(
                crate_name=name,
                version=v,
                checksum=cksum,  # 新增 checksum 参数
                mega_url=MEGA_URL,
                kafka_broker=KAFKA_BROKER,
                kafka_topic=KAFKA_TOPIC
            )
    
    save_processed(processed)

def get_all_files(index_dir):
    """遍历目录，返回所有文件信息"""
    crates = {}
    
    # 打印目录路径（调试用）
    print(f"正在扫描目录：{index_dir}")
    
    try:
        # 递归遍历所有子目录
        for root, dirs, files in os.walk(index_dir):
            if Path(root) == Path(index_dir):
                continue  # 跳过 index_dir 根目录（避免处理 config.json）
            for file in files:
                file_path = Path(os.path.join(root, file))
                if not file_path.is_file():
                    continue

                print(f"处理文件: {file_path}")  # 调试输出

                try:
                    with open(file_path, "r", encoding="utf-8") as f:
                        lines = f.readlines()
                        for line in lines:
                            try:
                                data = json.loads(line)
                                crate_name = data["name"]
                                version = data["vers"]
                                checksum = data["cksum"]  # 添加校验和
                                
                                # 如果这个 crate 还没有记录，初始化一个版本列表
                                if crate_name not in crates:
                                    crates[crate_name] = []
                                
                                # 添加这个版本的信息
                                crates[crate_name].append({
                                    "version": version,
                                    "description": data.get("desc", ""),
                                    "checksum": checksum  # 保存校验和
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
        # 分块读取文件以处理大文件
        for byte_block in iter(lambda: f.read(4096), b""):
            sha256_hash.update(byte_block)
    return sha256_hash.hexdigest()

def download_crate(crate_name, version, checksum):
    """
    下载指定的 crate 包并验证校验和
    
    Args:
        crate_name (str): crate 的名称
        version (str): crate 的版本
        checksum (str): 期望的 SHA-256 校验和
    
    Returns:
        bool: 下载是否成功
    """
    # 确保下载目录存在
    os.makedirs(DOWNLOAD_DIR, exist_ok=True)
    
    # 创建 crate 专属目录
    crate_dir = os.path.join(DOWNLOAD_DIR, crate_name)
    os.makedirs(crate_dir, exist_ok=True)
    
    # 构建下载 URL
    url = f"https://static.crates.io/crates/{crate_name}/{crate_name}-{version}.crate"
    
    # 构建保存路径
    save_path = os.path.join(crate_dir, f"{crate_name}-{version}.crate")
    
    # 如果文件已存在，验证校验和
    if os.path.exists(save_path):
        actual_checksum = calculate_sha256(save_path)
        if actual_checksum == checksum:
            print(f"文件已存在且校验和正确，跳过下载：{crate_name}-{version}.crate")
            return True
        else:
            print(f"文件已存在但校验和不匹配，将重新下载：{crate_name}-{version}.crate")
            os.remove(save_path)
    
    # 尝试下载，最多重试 MAX_RETRIES 次
    for attempt in range(MAX_RETRIES):
        try:
            # 使用 wget 下载文件
            subprocess.run([
                "wget",
                "-O", save_path,
                url
            ], check=True)
            
            # 验证校验和
            actual_checksum = calculate_sha256(save_path)
            if actual_checksum == checksum:
                print(f"成功下载并验证：{crate_name}-{version}.crate")
                return True
            else:
                print(f"校验和不匹配，尝试重新下载 (尝试 {attempt + 1}/{MAX_RETRIES})")
                os.remove(save_path)
                continue
                
        except subprocess.CalledProcessError as e:
            print(f"下载失败 {crate_name}-{version}.crate: {e}")
            if attempt < MAX_RETRIES - 1:
                print(f"尝试重新下载 (尝试 {attempt + 1}/{MAX_RETRIES})")
                continue
            return False
    
    print(f"达到最大重试次数，下载失败：{crate_name}-{version}.crate")
    return False


def decompress_crate(crate_path, extract_to=None):
    """
    解压 .crate 文件到当前目录（extract_to为None时为当前目录）
    """
    if extract_to is None:
        extract_to = os.getcwd()
    if not os.path.exists(extract_to):
        os.makedirs(extract_to)
    with tarfile.open(crate_path, "r:gz") as tar:
        tar.extractall(path=extract_to)

def init_or_clean_repo(repo_dir):
    if not os.path.exists(repo_dir):
        os.makedirs(repo_dir)
        subprocess.run(["git", "init"], cwd=repo_dir, check=True)
    """
    清空repo_dir下除.git以外的所有文件和文件夹
    """
    for item in os.listdir(repo_dir):
        if item == ".git":
            continue
        item_path = os.path.join(repo_dir, item)
        if os.path.isfile(item_path):
            os.remove(item_path)
        elif os.path.isdir(item_path):
            shutil.rmtree(item_path)


def copy_files(src_dir, dst_dir):
    """
    拷贝所有文件到目标目录
    """
    for item in os.listdir(src_dir):
        s = os.path.join(src_dir, item)
        d = os.path.join(dst_dir, item)
        if os.path.isdir(s):
            shutil.copytree(s, d, dirs_exist_ok=True)
        else:
            shutil.copy2(s, d)

def add_and_commit(repo_path: str, version: str):
    # 确保路径存在
    os.makedirs(repo_path, exist_ok=True)

    # 如果不是 git 仓库，就 init 一个
    if not os.path.exists(os.path.join(repo_path, '.git')):
        repo = Repo.init(repo_path)
    else:
        repo = Repo(repo_path)

    # 设置用户名和邮箱（等价于 Signature）
    author = Actor("Mega", "admin@mega.com")
    committer = Actor("Mega", "admin@mega.com")

    # 添加所有变更（包括首次 add）
    repo.index.add(["."])

    # 判断是否已有 commit
    if repo.head.is_valid():
        # 普通提交，有 parent commit
        repo.index.commit(f"Commit Version: {version}", author=author, committer=committer)
    else:
        # 初次提交，无 parent（空仓库）
        repo.index.commit(f"Commit Version: {version}", author=author, committer=committer)

    # 创建轻量 tag，如果 tag 已存在则忽略
    if version not in [tag.name for tag in repo.tags]:
        repo.create_tag(version)
    else:
        print(f"Tag '{version}' already exists.")

    print(f"Committed and tagged version {version} in {repo_path}")

def git_push(repo_dir, mega_url, branch="master"):
    """
    设置远程并推送
    """
    subprocess.run(["git", "remote", "remove", "nju"], cwd=repo_dir, check=False)
    print(f"git remote add nju {mega_url}")
    subprocess.run(["git", "remote", "add", "nju", mega_url], cwd=repo_dir, check=True)
    print(f"git push -f --set-upstream nju {branch}")
    try:
        subprocess.run(["git", "push", "-f", "--set-upstream", "nju", branch], cwd=repo_dir, check=False)
    except subprocess.CalledProcessError as e:
        print(f"错误输出: {e.stderr}")
    #subprocess.run(["git", "push", "--set-upstream", "nju", branch], cwd=repo_dir, check=True)
    try:
        subprocess.run(["git", "push", "nju", "--tags"], cwd=repo_dir, check=False)
    except subprocess.CalledProcessError:
        print("Some tags already exist in remote, continuing...")

def send_kafka_message(broker, topic, message_dict):
    producer = KafkaProducer(
        bootstrap_servers=[broker],
        value_serializer=lambda v: json.dumps(v).encode('utf-8')
    )
    producer.send(topic, message_dict)
    producer.flush()

def remove_extension(path):
    # 去掉扩展名，返回新路径
    base = os.path.splitext(path)[0]
    return base

def process_and_upload(crate_name, version, checksum, mega_url, kafka_broker, kafka_topic):
    crate_file = os.path.join(DOWNLOAD_DIR, crate_name, f"{crate_name}-{version}.crate")
    repo_dir = os.path.join(DOWNLOAD_DIR, crate_name, crate_name)
    crate_entry = os.path.join(DOWNLOAD_DIR, crate_name)

     # 1. 初始化/清空仓库
    init_or_clean_repo(repo_dir)

    # 2. 解压
    decompress_crate(crate_file, crate_entry)

    # 3. 去掉扩展名
    uncompress_path = remove_extension(crate_file)

    # 4. 拷贝文件
    copy_files(uncompress_path, repo_dir)

    # 5. 删除解压后的文件
    shutil.rmtree(uncompress_path)

    # 5. git commit/tag
    add_and_commit(repo_dir, version)

    # 6. git push
    mega_url_crate = f"{mega_url}/third-party/crates/{crate_name}"
    print(f"git push to {mega_url_crate}")
    git_push(repo_dir, mega_url_crate)

    # 7. 发送 kafka 消息
    message = {
        "crate_name": crate_name,
        "crate_version": version,
        "cksum": checksum,
        "data_source": "Manual",
        "timestamp": datetime.datetime.utcnow().isoformat()  + "Z" ,
        "version": "",
        "uuid": str(uuid.uuid4())
    }
    send_kafka_message(kafka_broker, kafka_topic, message)


def main():
    """
    主函数，协调整个下载过程
    """
    if clone_or_update_index():
        download_all_crates()
    else:
        print("初始化失败，退出程序")
        return 1
    return 0

if __name__ == "__main__":
    exit(main())