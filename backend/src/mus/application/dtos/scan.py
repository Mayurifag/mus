from typing import List, Optional

from pydantic import BaseModel, Field


class ScanRequestDTO(BaseModel):
    directory_paths: Optional[List[str]] = Field(
        default=None,
        description="Optional list of directory paths to scan. If not provided, the configured music directory will be used.",
    )

    clear_existing: bool = Field(
        default=False, description="Whether to clear existing tracks before scanning"
    )


class ScanProgressDTO(BaseModel):
    total_files: int = 0
    processed_files: int = 0
    added_tracks: int = 0
    skipped_files: int = 0
    errors: int = 0


class ScanResponseDTO(BaseModel):
    success: bool
    message: str
    tracks_changes: int = 0
    errors: int = 0
