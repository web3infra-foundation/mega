from sqlalchemy import Column, Integer, String, Text, DateTime, Enum, UniqueConstraint
from datetime import datetime
from .database import Base
import enum


class CrateTypeEnum(str, enum.Enum):
    LIB = "Lib"
    APPLICATION = "Application"

class SyncStatusEnum(str, enum.Enum):
    SYNCING = "Syncing"
    SUCCEED = "Succeed"
    FAILED = "Failed"
    ANALYSING = "Analysing"
    ANALYSED = "Analysed"

class RepoSyncResult(Base):
    __tablename__ = "repo_sync_result"

    id = Column(Integer, primary_key=True, index=True)
    crate_name = Column(String, unique=True, nullable=False)   # 唯一约束
    github_url = Column(Text, nullable=True)                   # 可空
    mega_url = Column(Text, nullable=False)                    # 必填
    status = Column(Enum(SyncStatusEnum), nullable=False)      # 枚举
    crate_type = Column(Enum(CrateTypeEnum), default=CrateTypeEnum.LIB, nullable=False)
    err_message = Column(Text, nullable=True)
    version = Column(String, nullable=False)
    created_at = Column(DateTime, default=datetime.utcnow, nullable=False)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow, nullable=False)

    def __repr__(self):
        return f"<RepoSyncResult(crate_name={self.crate_name}, status={self.status}, version={self.version})>"
