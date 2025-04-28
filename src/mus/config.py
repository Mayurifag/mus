import os
from pathlib import Path

# Database configuration
DATABASE_URL = "sqlite+aiosqlite:////app/data/mus.db"

# Music directory configuration
MUSIC_DIR = Path("/music")

# Ensure music directory exists
if not MUSIC_DIR.exists():
    os.makedirs(MUSIC_DIR, exist_ok=True)
