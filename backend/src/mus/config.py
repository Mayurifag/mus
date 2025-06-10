import os
from pathlib import Path
from typing import List

from pydantic import BaseModel, computed_field
from dotenv import load_dotenv

app_env = os.getenv("APP_ENV")
if app_env is None or app_env == "development":
    load_dotenv()

# Define BASE_DIR relative to this file (src/mus/config.py)
BASE_DIR = Path(__file__).resolve().parent


class Config(BaseModel):
    APP_ENV: str = app_env if app_env else "development"
    SECRET_KEY: str = os.getenv("SECRET_KEY", "default-insecure-secret-key")
    LOG_LEVEL: str = os.getenv("LOG_LEVEL", "INFO")
    SCAN_INTERVAL_SECONDS: int = int(os.getenv("SCAN_INTERVAL_SECONDS", "60"))
    MUSIC_DIR_PATH: Path = Path(str(BASE_DIR / ".." / ".." / "music"))

    CORS_ALLOWED_ORIGINS_STR: str = os.getenv(
        "CORS_ALLOWED_ORIGINS", "http://localhost:5173,http://127.0.0.1:5173"
    )

    @computed_field
    @property
    def CORS_ALLOWED_ORIGINS(self) -> List[str]:
        return [origin.strip() for origin in self.CORS_ALLOWED_ORIGINS_STR.split(",")]

    COVERS_DIR: str = os.getenv(
        "COVERS_DIR", str(BASE_DIR / ".." / ".." / "data" / "covers")
    )

    @computed_field
    @property
    def COVERS_DIR_PATH(self) -> Path:
        return Path(self.COVERS_DIR)

    STATIC_DIR: str = os.getenv("STATIC_DIR", str(BASE_DIR / ".." / ".." / "static"))

    @computed_field
    @property
    def STATIC_DIR_PATH(self) -> Path:
        return Path(self.STATIC_DIR)


settings = Config()
