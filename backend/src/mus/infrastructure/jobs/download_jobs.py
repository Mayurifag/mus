import logging

from src.mus.core.redis import get_redis_client
from src.mus.core.streaq_broker import worker


@worker.task()
async def download_track_from_url(url: str):
    logger = logging.getLogger(__name__)
    logger.info(f"WORKER: Starting download_track_from_url for URL: {url}")

    try:
        logger.info(f"WORKER: Placeholder job executed for URL: {url}")
    finally:
        client = await get_redis_client()
        try:
            lock_key = "download_lock:global"
            await client.delete(lock_key)
            logger.info("WORKER: Released download lock")
        finally:
            await client.aclose()

    logger.info(f"WORKER: Completed download_track_from_url for URL: {url}")
