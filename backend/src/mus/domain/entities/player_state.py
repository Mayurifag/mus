from typing import Optional
from sqlmodel import Field, SQLModel


class PlayerState(SQLModel, table=True):
    id: int = Field(default=1, primary_key=True)
    current_track_id: Optional[int] = Field(default=None, index=True)
    progress_seconds: float = Field(default=0.0)
    volume_level: float = Field(default=1.0)
    is_muted: bool = Field(default=False)
    is_shuffle: bool = Field(default=False)
    is_repeat: bool = Field(default=False)
