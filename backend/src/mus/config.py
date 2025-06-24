import os
from pathlib import Path
from typing import Optional

from pydantic import BaseModel, computed_field
from dotenv import load_dotenv

app_env = os.getenv("APP_ENV")
if app_env is None or app_env == "development":
    load_dotenv()


class Config(BaseModel):
    APP_ENV: str = app_env if app_env else "development"
    SECRET_KEY: Optional[str] = os.getenv("SECRET_KEY")
    LOG_LEVEL: str = os.getenv("LOG_LEVEL", "INFO")
    DRAGONFLY_URL: str = os.getenv("DRAGONFLY_URL", "redis://127.0.0.1:6379")

    DATA_DIR_PATH: Path = Path(os.getenv("DATA_DIR_PATH", "./app_data")).resolve()

    @computed_field
    @property
    def MUSIC_DIR_PATH(self) -> Path:
        return self.DATA_DIR_PATH / "music"

    FRONTEND_ORIGIN: str = os.getenv("FRONTEND_ORIGIN", "http://localhost:5173")

    @computed_field
    @property
    def COVERS_DIR_PATH(self) -> Path:
        return self.DATA_DIR_PATH / "covers"

    @computed_field
    @property
    def DATABASE_PATH(self) -> Path:
        return self.DATA_DIR_PATH / "database" / "mus.db"


settings = Config()
