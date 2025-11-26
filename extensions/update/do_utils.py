# db_utils.py
from datetime import datetime
from .database import SessionLocal
from .model import RepoSyncResult, SyncStatusEnum
from packaging.version import parse as vparse, InvalidVersion

def update_repo_sync_result(crate_name, version, mega_url, status: SyncStatusEnum, err_message=None):
    """插入或更新 repo_sync_result 表（对vparse不可解析的版本号强制覆盖）"""
    db = SessionLocal()
    try:
        record = db.query(RepoSyncResult).filter_by(crate_name=crate_name).first()
        
        # 标记当前版本是否可被vparse解析
        current_parsable = False
        try:
            current_version = vparse(version)
            current_parsable = True
        except InvalidVersion:
            print(f"版本号 {version}（{crate_name}）无法被vparse解析，将执行强制覆盖")
        
        # 标记数据库中已有版本是否可被vparse解析
        existing_parsable = False
        existing_version = None
        if record and record.version:
            try:
                existing_version = vparse(record.version)
                existing_parsable = True
            except InvalidVersion:
                print(f"数据库中 {crate_name} 的版本 {record.version} 无法被vparse解析")
        
        # 决定是否更新版本字段
        should_update_version = False
        if not record:
            # 新记录：直接写入
            should_update_version = True
        else:
            if not current_parsable:
                # 当前版本不可解析：强制覆盖
                should_update_version = True
            else:
                if not existing_parsable:
                    # 当前版本可解析，原有版本不可解析：覆盖
                    should_update_version = True
                else:
                    # 两者都可解析：按版本号大小比较
                    should_update_version = current_version > existing_version
        
        # 执行更新操作
        if record:
            if should_update_version:
                record.version = version
                record.mega_url = mega_url
            # 始终更新状态和时间
            record.status = status
            record.err_message = err_message
            record.updated_at = datetime.utcnow()
        else:
            record = RepoSyncResult(
                crate_name=crate_name,
                version=version,
                mega_url=mega_url,
                status=status,
                err_message=err_message,
                created_at=datetime.utcnow(),
                updated_at=datetime.utcnow(),
            )
            db.add(record)
        
        db.commit()
        print(f"成功处理 {crate_name}（版本 {version}）")
        
    except Exception as e:
        db.rollback()
        print(f"写入数据库失败: {e}")
    finally:
        db.close()


def load_processed_from_db():
    """从数据库读取已处理 crate 的最新版本及其 id"""
    db = SessionLocal()
    try:
        processed = {}
        records = db.query(RepoSyncResult).filter_by(status=SyncStatusEnum.SUCCEED).all()
        for r in records:
            processed[r.crate_name] = {
                "id": r.id,
                "latest_version": r.version
            }
        return processed
    finally:
        db.close()

