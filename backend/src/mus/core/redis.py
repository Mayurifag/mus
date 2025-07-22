import redis.asyncio as redis
from src.mus.config import settings

# Global Redis connection pool
_redis_pool = None


async def get_redis_client() -> redis.Redis:
    """Get a Redis client from the shared connection pool."""
    global _redis_pool
    if _redis_pool is None:
        _redis_pool = redis.ConnectionPool.from_url(settings.REDIS_URL)
    return redis.Redis(connection_pool=_redis_pool)


async def set_app_write_lock(file_path: str) -> None:
    """Set a Redis lock to prevent file watcher from processing app-initiated changes."""
    client = await get_redis_client()
    try:
        key = f"app_write_lock:{file_path}"
        await client.set(key, "1", ex=5)
    finally:
        await client.aclose()


async def check_app_write_lock(file_path: str) -> bool:
    """Check if there's an app write lock for this file path."""
    client = await get_redis_client()
    try:
        key = f"app_write_lock:{file_path}"
        result = await client.delete(key)
        return result > 0
    finally:
        await client.aclose()
