import os
from pydantic import BaseModel
from dotenv import load_dotenv

app_env = os.getenv("APP_ENV")
if app_env is None or app_env == "development":
    load_dotenv()


class Config(BaseModel):
    SECRET_KEY: str = os.getenv("SECRET_KEY", "default-insecure-secret-key")


settings = Config()
