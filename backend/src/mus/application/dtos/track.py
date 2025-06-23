from typing import Optional

from pydantic import BaseModel, Field


class TrackDTO(BaseModel):
    id: int
    title: str
    artist: str
    duration: int = Field(description="Duration in seconds")
    file_path: str
    added_at: int = Field(description="Unix timestamp when the track was added")
    has_cover: bool = False

    # URLs are constructed by the API layer based on track_id
    cover_small_url: Optional[str] = None
    cover_original_url: Optional[str] = None

    model_config = {"from_attributes": True}


class TrackListDTO(BaseModel):
    id: int
    title: str
    artist: str
    duration: int = Field(description="Duration in seconds")
    has_cover: bool = False

    # URLs are constructed by the API layer based on track_id
    cover_small_url: Optional[str] = None
    cover_original_url: Optional[str] = None

    model_config = {"from_attributes": True}
