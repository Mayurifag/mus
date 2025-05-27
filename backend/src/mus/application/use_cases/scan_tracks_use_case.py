from typing import (
    List,
    Optional,
    Dict,
    Any,
    Tuple,
    Callable,
)
from pathlib import Path
import asyncio
from mutagen.mp3 import MP3
from mutagen.flac import FLAC
import os
from datetime import datetime, timezone
from contextlib import AbstractAsyncContextManager
import logging

from sqlmodel.ext.asyncio.session import AsyncSession

from src.mus.domain.entities.track import Track
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor
from src.mus.application.dtos.scan import ScanResponseDTO
from src.mus.infrastructure.api.sse_handler import broadcast_sse_event

logger = logging.getLogger(__name__)


class ScanTracksUseCase:
    def __init__(
        self,
        session_factory: Callable[[], AbstractAsyncContextManager[AsyncSession]],
        file_system_scanner: FileSystemScanner,
        cover_processor: CoverProcessor,
    ):
        self.session_factory = session_factory
        self.file_system_scanner = file_system_scanner
        self.cover_processor = cover_processor
        self.batch_size = 20

    async def scan_directory(
        self, directory_paths: Optional[List[str]] = None
    ) -> ScanResponseDTO:
        tracks_changes = 0
        errors_total = 0

        scan_directories: Optional[List[Path]] = None
        if directory_paths:
            scan_directories = [Path(path) for path in directory_paths]

        async with self.session_factory() as session:
            track_repo = SQLiteTrackRepository(session)

            async with session.begin():
                latest_db_added_at = await track_repo.get_latest_track_added_at()

            min_mtime_for_scan = (
                int(latest_db_added_at) if latest_db_added_at is not None else None
            )

            all_files: List[Path] = []
            async for file_path in self.file_system_scanner.scan_directories(
                scan_directories, min_mtime=min_mtime_for_scan
            ):
                all_files.append(file_path)

            total_files_to_process = len(all_files)

            if not all_files:
                return ScanResponseDTO(
                    success=True,
                    message="Scan completed: No new or modified files found.",
                    tracks_changes=0,
                    errors=0,
                )

            for i in range(0, total_files_to_process, self.batch_size):
                batch = all_files[i : i + self.batch_size]
                async with self.session_factory() as batch_session:
                    batch_track_repo = SQLiteTrackRepository(batch_session)
                    (
                        added_in_batch,
                        errors_in_batch,
                    ) = await self._process_batch(batch, batch_track_repo)

                    tracks_changes += added_in_batch
                    errors_total += errors_in_batch

        summary_message = (
            f"Music library scan completed. {tracks_changes} items processed."
        )
        message_level = "success" if tracks_changes > 0 else "info"
        action_key = "reload_tracks" if tracks_changes > 0 else None

        asyncio.create_task(
            broadcast_sse_event(
                message_to_show=summary_message,
                message_level=message_level,
                action_key=action_key,
                action_payload=None,
            )
        )

        return ScanResponseDTO(
            success=True,
            message=f"Scan completed: {tracks_changes} changes, {errors_total} errors",
            tracks_changes=tracks_changes,
            errors=errors_total,
        )

    async def _process_batch(
        self,
        files: List[Path],
        track_repo: SQLiteTrackRepository,
    ) -> Tuple[int, int]:
        added_count = 0
        error_count = 0

        async with track_repo.session.begin():
            metadata_tasks = [
                asyncio.create_task(self._extract_metadata(file_path))
                for file_path in files
            ]

            tracks_to_process_covers: List[Tuple[int, Path]] = []

            for i, task in enumerate(metadata_tasks):
                file_path = files[i]
                try:
                    metadata = await task
                    if not metadata:
                        error_count += 1
                        logger.warning(f"Failed to extract metadata: {file_path}")
                        continue

                    track = Track(
                        title=metadata["title"],
                        artist=metadata["artist"],
                        duration=metadata["duration"],
                        file_path=str(file_path),
                        has_cover=False,
                        added_at=metadata.get(
                            "mtime", int(datetime.now(timezone.utc).timestamp())
                        ),
                    )

                    upserted_track = await track_repo.upsert_track(track)

                    if upserted_track.id is not None:
                        tracks_to_process_covers.append((upserted_track.id, file_path))

                    added_count += 1

                except Exception as e:
                    error_count += 1
                    logger.error(f"Error processing {file_path}: {str(e)}")

        if tracks_to_process_covers:
            cover_results = await self.cover_processor.process_tracks_covers_batch(
                tracks_to_process_covers
            )
            async with track_repo.session.begin():
                for track_id, success in cover_results.items():
                    if success:
                        await track_repo.set_cover_flag(track_id, True)

        return added_count, error_count

    async def _extract_metadata(self, file_path: Path) -> Optional[Dict[str, Any]]:
        try:
            return await asyncio.to_thread(self._extract_metadata_sync, file_path)
        except Exception as e:
            logger.warning(f"Metadata extraction failed for {file_path}: {e}")
            return None

    def _extract_metadata_sync(self, file_path: Path) -> Dict[str, Any]:
        try:
            file_ext = file_path.suffix.lower()
            mtime = int(os.path.getmtime(file_path))

            metadata = {
                "title": file_path.stem,
                "artist": "Unknown Artist",
                "duration": 0,
                "mtime": mtime,
            }

            if file_ext == ".mp3":
                audio = MP3(file_path)
                if audio.tags is not None:
                    title_frame = audio.tags.get("TIT2")
                    if (
                        title_frame
                        and hasattr(title_frame, "text")
                        and isinstance(title_frame.text, list)
                        and len(title_frame.text) > 0
                    ):
                        metadata["title"] = str(title_frame.text[0])

                    artist_frame = audio.tags.get("TPE1")
                    if (
                        artist_frame
                        and hasattr(artist_frame, "text")
                        and isinstance(artist_frame.text, list)
                        and len(artist_frame.text) > 0
                    ):
                        metadata["artist"] = str(artist_frame.text[0])

                if audio.info is not None:
                    metadata["duration"] = int(audio.info.length)

            elif file_ext == ".flac":
                audio = FLAC(file_path)
                title_list = audio.get("title")
                if title_list and isinstance(title_list, list) and title_list:
                    metadata["title"] = str(title_list[0])

                artist_list = audio.get("artist")
                if artist_list and isinstance(artist_list, list) and artist_list:
                    metadata["artist"] = str(artist_list[0])

                if audio.info is not None:
                    metadata["duration"] = int(audio.info.length)

            return metadata

        except Exception as e:
            logger.warning(
                f"Sync metadata extraction failed for {file_path}: {e}, using fallback."
            )
            fallback_mtime = int(datetime.now(timezone.utc).timestamp())
            try:
                fallback_mtime = int(os.path.getmtime(file_path))
            except OSError:
                pass

            return {
                "title": file_path.stem,
                "artist": "Unknown Artist",
                "duration": 0,
                "mtime": fallback_mtime,
            }
