import redis.asyncio as redis
from rq import Queue
from redis import Redis
from src.mus.config import settings

_redis_pool = None
_low_priority_queue = None
_high_priority_queue = None


async def get_redis_pool():
    global _redis_pool
    if _redis_pool is None:
        _redis_pool = redis.ConnectionPool.from_url(settings.DRAGONFLY_URL)
    return _redis_pool


async def set_app_write_lock(file_path: str) -> None:
    pool = await get_redis_pool()
    client = redis.Redis.from_pool(pool)
    try:
        key = f"app_write_lock:{file_path}"
        await client.set(key, "1", ex=5)
    finally:
        await client.aclose()


async def check_and_clear_app_write_lock(file_path: str) -> bool:
    pool = await get_redis_pool()
    client = redis.Redis.from_pool(pool)
    try:
        key = f"app_write_lock:{file_path}"
        result = await client.delete(key)
        return result > 0
    finally:
        await client.aclose()


def get_low_priority_queue() -> Queue:
    global _low_priority_queue
    if _low_priority_queue is None:
        _low_priority_queue = Queue(
            "low_priority", connection=Redis.from_url(settings.DRAGONFLY_URL)
        )
    return _low_priority_queue


def get_high_priority_queue() -> Queue:
    global _high_priority_queue
    if _high_priority_queue is None:
        _high_priority_queue = Queue(
            "high_priority", connection=Redis.from_url(settings.DRAGONFLY_URL)
        )
    return _high_priority_queue
