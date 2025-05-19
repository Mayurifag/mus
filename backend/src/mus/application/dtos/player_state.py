from typing import Optional

from pydantic import BaseModel, Field


class PlayerStateDTO(BaseModel):
    """DTO for PlayerState entity in API requests and responses."""

    current_track_id: Optional[int] = None
    progress_seconds: float = Field(
        default=0.0, ge=0.0, description="Current playback position in seconds"
    )
    volume_level: float = Field(
        default=1.0, ge=0.0, le=1.0, description="Volume level between 0.0 and 1.0"
    )
    is_muted: bool = False
    is_shuffle: bool = False
    is_repeat: bool = False

    model_config = {"from_attributes": True}
