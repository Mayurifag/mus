from typing import List, Optional

from pydantic import BaseModel, Field


class ScanRequestDTO(BaseModel):
    """DTO for scan request parameters."""

    directory_paths: Optional[List[str]] = Field(
        default=None,
        description="Optional list of directory paths to scan. If not provided, the configured music directory will be used.",
    )

    clear_existing: bool = Field(
        default=False, description="Whether to clear existing tracks before scanning"
    )


class ScanProgressDTO(BaseModel):
    """DTO for scan progress updates."""

    total_files: int = 0
    processed_files: int = 0
    added_tracks: int = 0
    skipped_files: int = 0
    errors: int = 0


class ScanResponseDTO(BaseModel):
    """DTO for scan response."""

    success: bool
    message: str
    tracks_added: int = 0
    tracks_updated: int = 0
    errors: int = 0
    error_details: Optional[List[str]] = None
