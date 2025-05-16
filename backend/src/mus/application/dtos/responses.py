from typing import Any, Dict, Optional

from pydantic import BaseModel


class StatusResponseDTO(BaseModel):
    """Generic status response DTO."""

    success: bool
    message: str


class ErrorResponseDTO(BaseModel):
    """Error response DTO with details."""

    success: bool = False
    message: str
    error_code: Optional[str] = None
    details: Optional[Dict[str, Any]] = None
