from typing import Any, Dict, Generic, List, Optional, TypeVar

from pydantic import BaseModel, Field


T = TypeVar("T")


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


class PagedResponseDTO(BaseModel, Generic[T]):
    """Paged response DTO for collections."""

    items: List[T]
    total: int
    page: int = 1
    page_size: int = Field(ge=1)
    has_more: bool = False
