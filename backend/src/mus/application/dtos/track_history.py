from typing import Optional
from pydantic import BaseModel, Field


class TrackHistoryDTO(BaseModel):
    id: int
    track_id: int
    title: str
    artist: str
    duration: int = Field(description="Duration in seconds")
    changed_at: int = Field(description="Unix timestamp when the change occurred")
    event_type: str = Field(
        description="Type of event (initial_scan, edit, metadata_update)"
    )
    filename: str = Field(description="Filename at the time of the change")
    changes: Optional[dict] = Field(
        default=None, description="Changes made to the track"
    )
    full_snapshot: Optional[dict] = Field(
        default=None, description="Complete metadata snapshot"
    )

    model_config = {"from_attributes": True}
