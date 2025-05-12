from fastapi import Depends
from typing import List, Optional
from pathlib import Path

from src.mus.domain.entities.track import Track
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor
from src.mus.application.dtos.scan import ScanResponseDTO


class ScanTracksUseCase:
    def __init__(
        self,
        track_repository: SQLiteTrackRepository = Depends(),
        file_system_scanner: FileSystemScanner = Depends(),
        cover_processor: CoverProcessor = Depends(),
    ):
        self.track_repository = track_repository
        self.file_system_scanner = file_system_scanner
        self.cover_processor = cover_processor

    async def scan_directory(
        self, directory_paths: Optional[List[str]] = None
    ) -> ScanResponseDTO:
        tracks_added = 0
        tracks_updated = 0
        errors = 0
        error_details: List[str] = []

        # Use default music directory if no paths provided
        scan_directories = [Path(self.file_system_scanner.MUSIC_DIR)]
        if directory_paths:
            scan_directories = [Path(path) for path in directory_paths]

        for directory in scan_directories:
            async for file_path in self.file_system_scanner.find_music_files(directory):
                try:
                    # Extract metadata from file
                    # This would normally come from a MetadataExtractor
                    # For simplicity, we're just using the file name as title and "Unknown" as artist
                    title = file_path.stem
                    artist = "Unknown"
                    duration = 0  # Should be extracted from file

                    # Create Track entity
                    track = Track(
                        title=title,
                        artist=artist,
                        duration=duration,
                        file_path=str(file_path),
                        has_cover=False,
                    )

                    # Use the upsert_track method to add or update the track
                    saved_track = await self.track_repository.upsert_track(track)

                    # Process cover art (if available)
                    # This is simplified - would normally extract cover art from the file
                    # For now, we just assume no cover art

                    # Update track counts based on whether this was an insert or update
                    # Note: This logic assumes that if the ID in saved_track matches what we
                    # provided, it's a new insert; otherwise, it was updated
                    is_new_track = track.id is None and saved_track.id is not None
                    if is_new_track:
                        tracks_added += 1
                    else:
                        tracks_updated += 1

                except Exception as e:
                    errors += 1
                    error_details.append(f"Error processing {file_path}: {str(e)}")

        return ScanResponseDTO(
            success=True,
            message=f"Scan completed: {tracks_added} added, {tracks_updated} updated, {errors} errors",
            tracks_added=tracks_added,
            tracks_updated=tracks_updated,
            errors=errors,
            error_details=error_details if error_details else None,
        )
