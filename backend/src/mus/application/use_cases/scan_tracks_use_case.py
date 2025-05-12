from fastapi import Depends
from typing import List, Optional, Dict, Any, Tuple
from pathlib import Path
import asyncio
from mutagen.mp3 import MP3
from mutagen.flac import FLAC

from src.mus.domain.entities.track import Track
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor
from src.mus.application.dtos.scan import ScanResponseDTO, ScanProgressDTO


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
        self.batch_size = 20  # Process 20 tracks at a time

    async def scan_directory(
        self, directory_paths: Optional[List[str]] = None
    ) -> ScanResponseDTO:
        tracks_added = 0
        tracks_updated = 0
        errors = 0
        error_details: List[str] = []

        # For tracking progress
        progress = ScanProgressDTO()

        # Convert string paths to Path objects
        scan_directories: Optional[List[Path]] = None
        if directory_paths:
            scan_directories = [Path(path) for path in directory_paths]

        # Collect all files first to determine total count
        all_files: List[Path] = []
        async for file_path in self.file_system_scanner.scan_directories(
            scan_directories
        ):
            all_files.append(file_path)

        progress.total_files = len(all_files)

        # Process files in batches
        for i in range(0, len(all_files), self.batch_size):
            batch = all_files[i : i + self.batch_size]
            batch_stats = await self._process_batch(batch, progress)

            tracks_added += batch_stats["added"]
            tracks_updated += batch_stats["updated"]
            errors += batch_stats["errors"]
            error_details.extend(batch_stats["error_details"])

            # Update progress
            progress.processed_files += len(batch)
            progress.added_tracks = tracks_added
            progress.errors = errors

        return ScanResponseDTO(
            success=True,
            message=f"Scan completed: {tracks_added} added, {tracks_updated} updated, {errors} errors",
            tracks_added=tracks_added,
            tracks_updated=tracks_updated,
            errors=errors,
            error_details=error_details if error_details else None,
        )

    async def _process_batch(
        self, files: List[Path], progress: ScanProgressDTO
    ) -> Dict[str, Any]:
        """Process a batch of files within a transaction"""
        batch_stats = {"added": 0, "updated": 0, "errors": 0, "error_details": []}

        # Begin transaction
        async with self.track_repository.session.begin():
            # Extract metadata and prepare tracks for upsert
            metadata_tasks = []
            for file_path in files:
                task = asyncio.create_task(self._extract_metadata(file_path))
                metadata_tasks.append((file_path, task))

            # Upsert tracks and collect track IDs for cover processing
            tracks_to_process: List[Tuple[int, Path]] = []

            for file_path, task in metadata_tasks:
                try:
                    metadata = await task
                    if not metadata:
                        batch_stats["errors"] += 1
                        batch_stats["error_details"].append(
                            f"Failed to extract metadata: {file_path}"
                        )
                        continue

                    # Create Track entity with extracted metadata
                    track = Track(
                        title=metadata["title"],
                        artist=metadata["artist"],
                        duration=metadata["duration"],
                        file_path=str(file_path),
                        has_cover=False,
                    )

                    # Upsert track
                    upserted_track = await self.track_repository.upsert_track(track)

                    # Add to list for cover processing if track has ID
                    if upserted_track.id is not None:
                        tracks_to_process.append((upserted_track.id, file_path))

                    # Determine if this was an add or update
                    if upserted_track.added_at == track.added_at:
                        batch_stats["added"] += 1
                    else:
                        batch_stats["updated"] += 1

                except Exception as e:
                    batch_stats["errors"] += 1
                    batch_stats["error_details"].append(
                        f"Error processing {file_path}: {str(e)}"
                    )

        # Process covers after the transaction completes (can be done outside transaction)
        if tracks_to_process:
            cover_results = await self.cover_processor.process_tracks_covers_batch(
                tracks_to_process
            )

            # Update has_cover flag for tracks with successfully processed covers
            async with self.track_repository.session.begin():
                for track_id, success in cover_results.items():
                    if success:
                        await self.track_repository.set_cover_flag(track_id, True)

        return batch_stats

    async def _extract_metadata(self, file_path: Path) -> Optional[Dict[str, Any]]:
        """Extract metadata from an audio file using mutagen"""
        try:
            # Run mutagen in a thread pool since it's CPU-bound
            return await asyncio.to_thread(self._extract_metadata_sync, file_path)
        except Exception:
            return None

    def _extract_metadata_sync(self, file_path: Path) -> Dict[str, Any]:
        """Synchronous implementation of metadata extraction"""
        try:
            file_ext = file_path.suffix.lower()

            # Default values
            metadata = {
                "title": file_path.stem,
                "artist": "Unknown",
                "duration": 0,
            }

            if file_ext == ".mp3":
                audio = MP3(file_path)

                # Extract title
                if audio.tags and "TIT2" in audio.tags:
                    metadata["title"] = str(audio.tags["TIT2"])

                # Extract artist
                if audio.tags and "TPE1" in audio.tags:
                    metadata["artist"] = str(audio.tags["TPE1"])

                # Extract duration (in seconds)
                if audio.info.length:
                    metadata["duration"] = int(audio.info.length)

            elif file_ext == ".flac":
                audio = FLAC(file_path)

                # Extract title
                if "title" in audio:
                    metadata["title"] = audio["title"][0]

                # Extract artist
                if "artist" in audio:
                    metadata["artist"] = audio["artist"][0]

                # Extract duration
                if audio.info.length:
                    metadata["duration"] = int(audio.info.length)

            return metadata

        except Exception:
            # In case of errors, return default metadata based on filename
            return {
                "title": file_path.stem,
                "artist": "Unknown",
                "duration": 0,
            }
