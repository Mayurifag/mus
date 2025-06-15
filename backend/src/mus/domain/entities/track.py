from sqlmodel import Field, SQLModel
from typing import Optional


class Track(SQLModel, table=True):
    id: Optional[int] = Field(default=None, primary_key=True)
    title: str
    artist: str
    duration: int
    file_path: str = Field(unique=True, index=True)
    added_at: int
    has_cover: bool = Field(default=False)
