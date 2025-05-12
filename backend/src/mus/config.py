import os
from pydantic import BaseModel
from dotenv import load_dotenv

load_dotenv()


class Config(BaseModel):
    SECRET_KEY: str = os.getenv("SECRET_KEY", "default-insecure-secret-key")


settings = Config()
