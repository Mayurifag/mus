import logging

from src.mus.core.redis import get_redis_client
from src.mus.core.streaq_broker import worker
from src.mus.infrastructure.api.sse_handler import notify_sse_from_worker


@worker.task()
async def download_track_from_url(url: str):
    logger = logging.getLogger(__name__)
    logger.info(f"WORKER: Starting download_track_from_url for URL: {url}")

    # Notify that download has started
    await notify_sse_from_worker(
        action_key="download_started",
        message="Download started",
        level="info",
        payload={"url": url},
    )

    try:
        # TODO: Implement actual download logic here
        logger.info(f"WORKER: Placeholder job executed for URL: {url}")

        # For now, simulate a failure since the actual download isn't implemented
        await notify_sse_from_worker(
            action_key="download_failed",
            message="Download functionality not yet implemented",
            level="error",
            payload={"error": "Download functionality not yet implemented"},
        )

    except Exception as e:
        logger.error(
            f"WORKER: Error in download_track_from_url for URL {url}: {str(e)}"
        )

        await notify_sse_from_worker(
            action_key="download_failed",
            message=f"Download failed: {str(e)}",
            level="error",
            payload={"error": str(e)},
        )

    finally:
        client = await get_redis_client()
        try:
            lock_key = "download_lock:global"
            await client.delete(lock_key)
            logger.info("WORKER: Released download lock")
        finally:
            await client.aclose()

    logger.info(f"WORKER: Completed download_track_from_url for URL: {url}")
