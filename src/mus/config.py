import os
from pathlib import Path


# Database configuration
def get_database_url() -> str:
    db_path = os.getenv("DATABASE_PATH", "/app/data/mus.db")
    return f"sqlite+aiosqlite:///{db_path}"


# Music directory configuration
def get_music_dir() -> Path:
    music_dir = Path(os.getenv("MUSIC_DIR", "/music"))
    if not music_dir.exists():
        os.makedirs(music_dir, exist_ok=True)
    return music_dir
