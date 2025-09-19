# db_utils.py
from datetime import datetime
from .database import SessionLocal
from .model import RepoSyncResult, SyncStatusEnum
from packaging.version import parse as vparse

def update_repo_sync_result(crate_name, version, mega_url, status: SyncStatusEnum, err_message=None):
    """插入或更新 repo_sync_result 表"""
    db = SessionLocal()
    try:
        record = db.query(RepoSyncResult).filter_by(crate_name=crate_name).first()
        if record:
            if record.version is None or vparse(version) > vparse(record.version):
                record.version = version
                record.mega_url = mega_url
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

