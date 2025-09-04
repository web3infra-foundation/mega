from pydantic import BaseModel
from typing import Optional
# from datetime import datetime
from enum import Enum
from .model import CrateTypeEnum, SyncStatusEnum

class MessageKindEnum(str, Enum):
    MEGA = "Mega"
    USER = "User"

class SourceOfDataEnum(str, Enum):
    CRATESIO = "Cratesio"
    GITHUB = "Github"


# -------------------
# 数据库模型对应（简化版，用于消息封装）
# -------------------

class RepoSyncModel(BaseModel):
    id: int
    crate_name: str
    github_url: Optional[str]
    mega_url: str
    crate_type: CrateTypeEnum
    status: SyncStatusEnum
    err_message: Optional[str] = None
    # 如果消息里也需要时间，可以加：
    # created_at: Optional[datetime] = None
    # updated_at: Optional[datetime] = None


# -------------------
# Kafka 消息模型
# -------------------

class MessageModel(BaseModel):
    db_model: RepoSyncModel
    message_kind: MessageKindEnum
    source_of_data: SourceOfDataEnum
    timestamp: str
    extra_field: Optional[str] = None
