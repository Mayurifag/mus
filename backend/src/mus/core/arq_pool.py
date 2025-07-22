from arq import ArqRedis, create_pool
from arq.connections import RedisSettings
from src.mus.config import settings


async def get_arq_pool() -> ArqRedis:
    redis_settings = RedisSettings.from_dsn(settings.DRAGONFLY_URL)
    return await create_pool(redis_settings)
