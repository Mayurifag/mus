from streaq import Worker

from src.mus.config import settings

worker = Worker(redis_url=settings.DRAGONFLY_URL)
