from pathlib import Path
from typing import Optional
from fastapi import HTTPException, UploadFile
from src.mus.config import settings


SUPPORTED_AUDIO_EXTENSIONS = {".mp3", ".flac", ".wav"}


def validate_file_size(file: UploadFile, max_size_mb: Optional[int] = None) -> None:
    """Validate file size against maximum allowed size."""
    max_size_mb = max_size_mb or settings.MAX_UPLOAD_SIZE_MB
    max_size_bytes = max_size_mb * 1024 * 1024

    if file.size and file.size > max_size_bytes:
        raise HTTPException(
            status_code=413,
            detail=f"File size exceeds maximum allowed size ({max_size_mb}MB)",
        )


def validate_audio_extension(filename: str) -> str:
    """Validate file extension and return normalized extension."""
    extension = Path(filename).suffix.lower()

    if extension not in SUPPORTED_AUDIO_EXTENSIONS:
        raise HTTPException(
            status_code=400,
            detail="Unsupported file format. Only MP3, FLAC, and WAV are supported.",
        )

    return extension


def validate_upload_file(file: UploadFile) -> str:
    """Comprehensive validation for uploaded audio files."""
    validate_file_size(file)
    filename = file.filename or "upload"
    return validate_audio_extension(filename)
