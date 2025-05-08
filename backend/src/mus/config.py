from pydantic import BaseModel
import os
from dotenv import load_dotenv

load_dotenv()


class Config(BaseModel):
    SECRET_KEY: str = os.getenv("SECRET_KEY", "default-insecure-secret-key")
    LOGIN_SECRET: str = os.getenv("LOGIN_SECRET", "default-login-secret")
    DATABASE_URL: str = os.getenv("DATABASE_URL", "sqlite+aiosqlite:///data/mus.db")
    MUSIC_DIR: str = os.getenv("MUSIC_DIR", "./music")
    COVERS_DIR: str = os.getenv("COVERS_DIR", "./data/covers")


settings = Config()
