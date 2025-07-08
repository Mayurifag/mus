from enum import Enum
from sqlmodel import Field, SQLModel
from sqlalchemy import Column, String
from typing import Optional


class ProcessingStatus(str, Enum):
    PENDING = "PENDING"
    METADATA_DONE = "METADATA_DONE"
    COMPLETE = "COMPLETE"
    ERROR = "ERROR"


class Track(SQLModel, table=True):
    id: Optional[int] = Field(default=None, primary_key=True)
    title: str
    artist: str
    duration: int
    file_path: str = Field(unique=True, index=True)
    added_at: int
    updated_at: int = Field(default=0)
    has_cover: bool = Field(default=False)
    inode: Optional[int] = Field(default=None, index=True)
    content_hash: Optional[str] = Field(default=None, index=True)
    processing_status: ProcessingStatus = Field(
        default=ProcessingStatus.PENDING, sa_column=Column(String(20), nullable=False)
    )
    last_error_message: Optional[str] = Field(default=None)
