from pydantic import BaseModel, Field


class TrackHistoryDTO(BaseModel):
    id: int
    track_id: int
    title: str
    artist: str
    duration: int = Field(description="Duration in seconds")
    changed_at: int = Field(description="Unix timestamp when the change occurred")

    model_config = {"from_attributes": True}
