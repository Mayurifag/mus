import os
from pathlib import Path
from typing import Optional

from pydantic import BaseModel, computed_field
from dotenv import load_dotenv

app_env = os.getenv("APP_ENV")
if app_env is None or app_env == "development":
    load_dotenv()

# Define BASE_DIR relative to this file (src/mus/config.py)
BASE_DIR = Path(__file__).resolve().parent


class Config(BaseModel):
    APP_ENV: str = app_env if app_env else "development"
    SECRET_KEY: Optional[str] = os.getenv("SECRET_KEY")
    LOG_LEVEL: str = os.getenv("LOG_LEVEL", "INFO")
    SCAN_INTERVAL_SECONDS: int = int(os.getenv("SCAN_INTERVAL_SECONDS", "60"))

    @computed_field
    @property
    def MUSIC_DIR_PATH(self) -> Path:
        music_dir = os.getenv("MUSIC_DIR_PATH")
        if music_dir:
            return Path(music_dir).resolve()
        return (BASE_DIR / ".." / ".." / "music").resolve()

    FRONTEND_ORIGIN: str = os.getenv("FRONTEND_ORIGIN", "http://localhost:5173")

    COVERS_DIR: str = os.getenv(
        "COVERS_DIR", str(BASE_DIR / ".." / ".." / "data" / "covers")
    )

    @computed_field
    @property
    def COVERS_DIR_PATH(self) -> Path:
        return Path(self.COVERS_DIR).resolve()


settings = Config()
