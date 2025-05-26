import os
from pathlib import Path
from typing import List

from pydantic import BaseModel, field_validator, computed_field
from pydantic_core.core_schema import ValidationInfo
from dotenv import load_dotenv

app_env = os.getenv("APP_ENV")
if app_env is None or app_env == "development":
    load_dotenv()

# Define BASE_DIR relative to this file (src/mus/config.py)
# So BASE_DIR will be src/
BASE_DIR = Path(__file__).resolve().parent


class Config(BaseModel):
    APP_ENV: str = app_env if app_env else "development"
    SECRET_KEY: str = os.getenv("SECRET_KEY", "default-insecure-secret-key")
    LOG_LEVEL: str = os.getenv("LOG_LEVEL", "INFO")
    SCAN_INTERVAL_SECONDS: int = int(os.getenv("SCAN_INTERVAL_SECONDS", "60"))

    CORS_ALLOWED_ORIGINS_STR: str = os.getenv(
        "CORS_ALLOWED_ORIGINS", "http://localhost:5173,http://127.0.0.1:5173"
    )

    @computed_field
    @property
    def CORS_ALLOWED_ORIGINS(self) -> List[str]:
        return [origin.strip() for origin in self.CORS_ALLOWED_ORIGINS_STR.split(",")]

    # Define paths relative to BASE_DIR (src/mus/) or allow full paths from env
    # Defaulting to project structure: backend/data/covers, backend/static, backend/music
    # config.py is at backend/src/mus/config.py
    # BASE_DIR is backend/src/mus/
    # BASE_DIR.parent is backend/src/
    # BASE_DIR.parent.parent is backend/
    # BASE_DIR.parent.parent.parent is the project root /Users/mayurifag/Code/mus
    PROJECT_ROOT: Path = BASE_DIR.parent.parent.parent

    COVERS_DIR: str = os.getenv(
        "COVERS_DIR", str(PROJECT_ROOT / "backend" / "data" / "covers")
    )

    @computed_field
    @property
    def COVERS_DIR_PATH(self) -> Path:
        return Path(self.COVERS_DIR)

    STATIC_DIR: str = os.getenv("STATIC_DIR", str(PROJECT_ROOT / "backend" / "static"))

    @computed_field
    @property
    def STATIC_DIR_PATH(self) -> Path:
        return Path(self.STATIC_DIR)

    MUSIC_DIRECTORIES_STR: str = os.getenv(
        "MUSIC_DIRECTORIES", str(PROJECT_ROOT / "backend" / "music")
    )
    MUSIC_DIRECTORIES_LIST: List[Path] = []

    @field_validator("MUSIC_DIRECTORIES_LIST", mode="before")
    def assemble_music_directories(
        cls, v: List[Path], info: ValidationInfo
    ) -> List[Path]:
        values = info.data
        music_dirs_str = values.get("MUSIC_DIRECTORIES_STR")
        if isinstance(music_dirs_str, str):
            return [Path(p.strip()) for p in music_dirs_str.split(",")]
        return v


settings = Config()
