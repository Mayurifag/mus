"""Data Transfer Objects for API requests and responses."""

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

__all__ = [
    "TrackDTO",
    "PlayerStateDTO",
    "ScanRequestDTO",
    "ScanProgressDTO",
    "ScanResponseDTO",
    "StatusResponseDTO",
    "ErrorResponseDTO",
    "PagedResponseDTO",
]
