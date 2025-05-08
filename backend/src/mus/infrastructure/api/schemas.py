"""
API schemas file for FastAPI endpoints.

This file re-exports DTOs from the application layer with API-specific naming.
"""

from src.mus.application.dtos.player_state import PlayerStateDTO
from src.mus.application.dtos.responses import (
    ErrorResponseDTO,
    PagedResponseDTO,
    StatusResponseDTO,
)
from src.mus.application.dtos.scan import (
    ScanProgressDTO,
    ScanRequestDTO,
    ScanResponseDTO,
)
from src.mus.application.dtos.track import TrackDTO

# Re-export with API-specific names
Track = TrackDTO
PlayerState = PlayerStateDTO

# Scan schemas
ScanRequest = ScanRequestDTO
ScanProgress = ScanProgressDTO
ScanResponse = ScanResponseDTO

# Generic response schemas
StatusResponse = StatusResponseDTO
ErrorResponse = ErrorResponseDTO

# Collections
TrackList = PagedResponseDTO[TrackDTO]
