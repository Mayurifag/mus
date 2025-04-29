from pathlib import Path

import aiosqlite
import structlog

from mus.domain.repositories.track_repository import ITrackRepository
from mus.domain.track import Track

logger = structlog.get_logger()


class SQLiteTrackRepository(ITrackRepository):
    def __init__(self, db_path: str):
        self.db_path = db_path

    async def _init_db(self):
        async with aiosqlite.connect(self.db_path) as db:
            await db.execute(
                """
                CREATE TABLE IF NOT EXISTS tracks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL,
                    artist TEXT NOT NULL,
                    duration INTEGER NOT NULL,
                    file_path TEXT NOT NULL UNIQUE,
                    added_at INTEGER NOT NULL
                )
                """
            )
            await db.commit()

    async def add(self, track: Track) -> None:
        await self._init_db()
        async with aiosqlite.connect(self.db_path) as db:
            await db.execute(
                """
                INSERT INTO tracks (title, artist, duration, file_path, added_at)
                VALUES (?, ?, ?, ?, ?)
                """,
                (
                    track.title,
                    track.artist,
                    track.duration,
                    str(track.file_path),
                    track.added_at,
                ),
            )
            await db.commit()

    async def exists_by_path(self, file_path: Path) -> bool:
        await self._init_db()
        async with aiosqlite.connect(self.db_path) as db:
            async with db.execute(
                "SELECT 1 FROM tracks WHERE file_path = ?", (str(file_path),)
            ) as cursor:
                result = await cursor.fetchone()
                return result is not None

    async def search_by_title(self, query: str) -> list[Track]:
        await self._init_db()
        async with aiosqlite.connect(self.db_path) as db:
            async with db.execute(
                "SELECT * FROM tracks WHERE title LIKE ?", (f"%{query}%",)
            ) as cursor:
                rows = await cursor.fetchall()
                return [
                    Track(
                        title=row[1],
                        artist=row[2],
                        duration=row[3],
                        file_path=Path(row[4]),
                        added_at=row[5],
                    )
                    for row in rows
                ]

    async def get_all(self) -> list[Track]:
        await self._init_db()
        async with aiosqlite.connect(self.db_path) as db:
            async with db.execute(
                "SELECT * FROM tracks ORDER BY added_at DESC"
            ) as cursor:
                rows = await cursor.fetchall()
                return [
                    Track(
                        title=row[1],
                        artist=row[2],
                        duration=row[3],
                        file_path=Path(row[4]),
                        added_at=row[5],
                    )
                    for row in rows
                ]

    async def clear_all_tracks(self) -> None:
        await self._init_db()
        async with aiosqlite.connect(self.db_path) as db:
            await db.execute("DELETE FROM tracks")
            await db.commit()
        logger.info("All tracks cleared from repository")
