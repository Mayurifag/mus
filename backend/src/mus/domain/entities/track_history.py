from sqlmodel import Field, SQLModel
from typing import Optional


class TrackHistory(SQLModel, table=True):
    id: Optional[int] = Field(default=None, primary_key=True)
    track_id: int = Field(foreign_key="track.id", index=True)
    title: str
    artist: str
    duration: int
    changed_at: int
