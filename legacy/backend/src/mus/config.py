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


# Covers directory configuration
def get_covers_dir() -> Path:
    covers_dir = Path(os.getenv("COVERS_DIR", "/app/data/covers"))
    if not covers_dir.exists():
        os.makedirs(covers_dir, exist_ok=True)
    return covers_dir


# Authentication configuration
def get_secret_auth_route() -> str | None:
    secret_route = os.getenv("SECRET_AUTH_ROUTE")
    return secret_route.strip("/") if secret_route else None


def get_session_cookie_secret() -> str | None:
    return get_secret_auth_route()
