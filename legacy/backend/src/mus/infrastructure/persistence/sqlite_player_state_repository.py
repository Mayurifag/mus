import sqlite3

from mus.application.ports.player_state_repository import IPlayerStateRepository
from mus.domain.player_state import PlayerState


class SQLitePlayerStateRepository(IPlayerStateRepository):
    def __init__(self, db_path: str):
        self._db_path = db_path
        self._init_db()

    def _init_db(self) -> None:
        with sqlite3.connect(self._db_path) as conn:
            conn.execute("""
                CREATE TABLE IF NOT EXISTS player_state (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    current_track_id INTEGER,
                    progress_seconds REAL DEFAULT 0.0,
                    volume_level REAL DEFAULT 1.0,
                    is_muted BOOLEAN DEFAULT 0,
                    updated_at INTEGER DEFAULT (unixepoch()),
                    FOREIGN KEY (current_track_id)
                        REFERENCES tracks(id) ON DELETE SET NULL
                )
            """)

    async def save_state(self, state: PlayerState) -> None:
        with sqlite3.connect(self._db_path) as conn:
            conn.execute(
                """
                INSERT OR REPLACE INTO player_state
                (id, current_track_id, progress_seconds,
                 volume_level, is_muted, updated_at)
                VALUES (1, ?, ?, ?, ?, unixepoch())
            """,
                (
                    state.current_track_id,
                    state.progress_seconds,
                    state.volume_level,
                    state.is_muted,
                ),
            )

    async def load_state(self) -> PlayerState | None:
        with sqlite3.connect(self._db_path) as conn:
            conn.row_factory = sqlite3.Row
            cursor = conn.execute("SELECT * FROM player_state WHERE id = 1")
            row = cursor.fetchone()

            if row is None:
                return None

            return PlayerState(
                current_track_id=row["current_track_id"],
                progress_seconds=row["progress_seconds"],
                volume_level=row["volume_level"],
                is_muted=bool(row["is_muted"]),
            )
