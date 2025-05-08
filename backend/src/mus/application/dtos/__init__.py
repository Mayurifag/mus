"""Data Transfer Objects for API requests and responses."""

from mus.application.dtos.player_state import PlayerStateDTO
from mus.application.dtos.responses import (
    ErrorResponseDTO,
    PagedResponseDTO,
    StatusResponseDTO,
)
from mus.application.dtos.scan import (
    ScanProgressDTO,
    ScanRequestDTO,
    ScanResponseDTO,
)
from mus.application.dtos.track import TrackDTO

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
