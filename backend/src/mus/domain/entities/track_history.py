from sqlmodel import Field, SQLModel
from sqlalchemy import Column, JSON
from typing import Optional


class TrackHistory(SQLModel, table=True):
    id: Optional[int] = Field(default=None, primary_key=True)
    track_id: int = Field(foreign_key="track.id", index=True)
    title: str
    artist: str
    duration: int
    changed_at: int
    changes: Optional[dict] = Field(default=None, sa_column=Column(JSON))
    full_snapshot: Optional[dict] = Field(default=None, sa_column=Column(JSON))
    filename: str
    event_type: str
