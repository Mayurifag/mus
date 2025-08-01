from typing import Optional

from pydantic import BaseModel, Field


class TrackDTO(BaseModel):
    id: int
    title: str
    artist: str
    duration: int = Field(description="Duration in seconds")
    file_path: str
    added_at: int = Field(description="Unix timestamp when the track was added")
    updated_at: int = Field(
        description="Unix timestamp when the track was last updated"
    )
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
    file_path: str
    updated_at: int = Field(
        description="Unix timestamp when the track was last updated"
    )
    has_cover: bool = False

    # URLs are constructed by the API layer based on track_id
    cover_small_url: Optional[str] = None
    cover_original_url: Optional[str] = None

    model_config = {"from_attributes": True}


class TrackUpdateDTO(BaseModel):
    title: Optional[str] = None
    artist: Optional[str] = None
    rename_file: bool = False
