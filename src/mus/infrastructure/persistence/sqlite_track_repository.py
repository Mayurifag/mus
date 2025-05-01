from pathlib import Path

import aiosqlite
import structlog
from aiosqlite import Row

from mus.domain.repositories.track_repository import ITrackRepository
from mus.domain.track import Track

logger = structlog.get_logger()

# Column indices in the tracks table
ID_COL = 0
TITLE_COL = 1
ARTIST_COL = 2
DURATION_COL = 3
FILE_PATH_COL = 4
ADDED_AT_COL = 5
HAS_COVER_COL = 6


class SQLiteTrackRepository(ITrackRepository):
    def __init__(self, db_path: str):
        self.db_path = db_path

    async def initialize_schema(self) -> None:
        """Initialize the database schema, creating tables and indices."""
        await self._init_db()

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
                    added_at INTEGER NOT NULL,
                    has_cover BOOLEAN DEFAULT 0
                )
                """
            )
            # Create index on added_at for faster sorting
            await db.execute(
                """
                CREATE INDEX IF NOT EXISTS idx_tracks_added_at ON tracks(added_at DESC)
                """
            )
            # Add has_cover column if it doesn't exist (for backward compatibility)
            try:
                await db.execute(
                    "ALTER TABLE tracks ADD COLUMN has_cover BOOLEAN DEFAULT 0"
                )
                await db.commit()
                logger.info("Added has_cover column to tracks table")
            except aiosqlite.OperationalError:
                # Column already exists
                pass

    async def add(self, track: Track) -> int:
        await self._init_db()
        async with aiosqlite.connect(self.db_path) as db:
            cursor = await db.execute(
                """
                INSERT INTO tracks
                (title, artist, duration, file_path, added_at, has_cover)
                VALUES (?, ?, ?, ?, ?, ?)
                """,
                (
                    track.title,
                    track.artist,
                    track.duration,
                    str(track.file_path),
                    track.added_at,
                    track.has_cover,
                ),
            )
            await db.commit()
            if cursor.lastrowid is None:
                raise RuntimeError("Failed to get ID of inserted track")
            return cursor.lastrowid

    async def set_cover_flag(self, track_id: int, has_cover: bool) -> None:
        async with aiosqlite.connect(self.db_path) as db:
            await db.execute(
                "UPDATE tracks SET has_cover = ? WHERE id = ?",
                (has_cover, track_id),
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
                return [self._row_to_track(row) for row in rows]

    async def get_all(self) -> list[Track]:
        await self._init_db()
        async with aiosqlite.connect(self.db_path) as db:
            async with db.execute(
                "SELECT * FROM tracks ORDER BY added_at DESC"
            ) as cursor:
                rows = await cursor.fetchall()
                return [self._row_to_track(row) for row in rows]

    async def clear_all_tracks(self) -> None:
        await self._init_db()
        async with aiosqlite.connect(self.db_path) as db:
            await db.execute("DELETE FROM tracks")
            await db.commit()
        logger.info("All tracks cleared from repository")

    async def get_by_id(self, track_id: int) -> Track | None:
        await self._init_db()
        async with aiosqlite.connect(self.db_path) as db:
            async with db.execute(
                """
                SELECT id, title, artist, duration, file_path, added_at, has_cover
                FROM tracks
                WHERE id = ?
                """,
                (track_id,),
            ) as cursor:
                row = await cursor.fetchone()
                if row is None:
                    return None
                return self._row_to_track(row)

    def _row_to_track(self, row: Row | tuple) -> Track:
        """Convert a database row to a Track object.

        Args:
            row: A database row, either as a tuple or aiosqlite.Row.

        Returns:
            Track: A Track object created from the row data.
        """
        return Track(
            id=row[ID_COL],
            title=row[TITLE_COL],
            artist=row[ARTIST_COL],
            duration=row[DURATION_COL],
            file_path=Path(row[FILE_PATH_COL]),
            added_at=row[ADDED_AT_COL],
            has_cover=bool(row[HAS_COVER_COL]) if len(row) > HAS_COVER_COL else False,
        )
