import asyncio
import time
import logging
from pathlib import Path

from src.mus.domain.entities.track import Track, ProcessingStatus
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.infrastructure.persistence.batch_operations import batch_upsert_tracks
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner
from src.mus.infrastructure.database import async_session_factory, create_db_and_tables
from src.mus.util.metadata_extractor import extract_fast_metadata
from src.mus.config import settings

logger = logging.getLogger(__name__)

BATCH_SIZE = 100


class InitialScanner:
    def __init__(
        self, track_repository: SQLiteTrackRepository, batch_size: int = BATCH_SIZE
    ):
        self.track_repository = track_repository
        self.file_scanner = FileSystemScanner(settings.MUSIC_DIR_PATH)
        self.batch_size = batch_size

    @classmethod
    async def create_default(cls) -> "InitialScanner":
        settings.DATABASE_PATH.parent.mkdir(parents=True, exist_ok=True)
        settings.COVERS_DIR_PATH.mkdir(parents=True, exist_ok=True)
        settings.MUSIC_DIR_PATH.mkdir(parents=True, exist_ok=True)
        await create_db_and_tables()

        async with async_session_factory() as session:
            track_repository = SQLiteTrackRepository(session)
            return cls(track_repository)

    async def scan(self) -> "InitialScanner":
        tasks_with_paths: list[tuple[asyncio.Task, Path]] = []

        async for file_path in self.file_scanner.scan_directories():
            tasks_with_paths.append(
                (
                    asyncio.create_task(
                        asyncio.to_thread(extract_fast_metadata, file_path)
                    ),
                    file_path,
                )
            )

            if len(tasks_with_paths) >= self.batch_size:
                await self._process_tasks_batch(tasks_with_paths)
                tasks_with_paths = []

        if tasks_with_paths:
            await self._process_tasks_batch(tasks_with_paths)

        return self

    async def _process_tasks_batch(
        self, tasks_with_paths: list[tuple[asyncio.Task, Path]]
    ) -> None:
        now = int(time.time())
        metadata_results = await asyncio.gather(*(t for t, _ in tasks_with_paths))

        tracks = [
            Track(
                title=m["title"],
                artist=m["artist"],
                duration=m["duration"],
                file_path=str(p),
                added_at=m.get("added_at", now),
                inode=m["inode"],
                processing_status=ProcessingStatus.METADATA_DONE,
            )
            for m, (_, p) in zip(metadata_results, tasks_with_paths)
            if m
        ]

        if tracks:
            await batch_upsert_tracks(self.track_repository.session, tracks)
