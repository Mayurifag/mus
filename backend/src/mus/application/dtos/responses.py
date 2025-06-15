from typing import Any, Dict, Optional

from pydantic import BaseModel


class StatusResponseDTO(BaseModel):
    success: bool
    message: str


class ErrorResponseDTO(BaseModel):
    success: bool = False
    message: str
    error_code: Optional[str] = None
    details: Optional[Dict[str, Any]] = None
