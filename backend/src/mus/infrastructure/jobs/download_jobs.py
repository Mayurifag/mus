import logging
import shutil
import tempfile
from pathlib import Path
from urllib.parse import urlparse

from src.mus.config import settings
from src.mus.core.redis import get_redis_client, set_app_write_lock
from src.mus.core.streaq_broker import worker
from src.mus.infrastructure.api.sse_handler import notify_sse_from_worker
from src.mus.infrastructure.jobs.file_system_jobs import handle_file_created
from src.mus.infrastructure.services.ytdlp_service import ytdlp_service


def _validate_url(url: str) -> bool:
    try:
        parsed = urlparse(url)
        return parsed.scheme in ["http", "https"] and bool(parsed.netloc)
    except Exception:
        return False


@worker.task()
async def download_track_from_url(url: str):
    logger = logging.getLogger(__name__)
    logger.info(f"WORKER: Starting download for URL: {url}")

    await notify_sse_from_worker(
        action_key="download_started",
        message="Download started",
        level="info",
        payload={"url": url},
    )

    try:
        if not _validate_url(url):
            raise ValueError("Invalid URL format")

        output_path = await _download_audio(url, logger)

        await set_app_write_lock(output_path)

        async with worker:
            await handle_file_created.enqueue(
                file_path_str=output_path,
                skip_slow_metadata=False,
            )

        await notify_sse_from_worker(
            action_key="download_completed",
            message="Download completed successfully",
            level="success",
            payload={"file_path": output_path},
        )

    except Exception as e:
        logger.error(f"WORKER: Download failed for URL {url}: {str(e)}")

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

    logger.info(f"WORKER: Completed download for URL: {url}")


async def _download_audio(url: str, logger: logging.Logger) -> str:
    with tempfile.TemporaryDirectory(prefix="mus-download-") as temp_dir_str:
        temp_dir = Path(temp_dir_str)
        logger.info(f"WORKER: Using temporary directory for download: {temp_dir}")

        output_template = str(
            temp_dir / "%(artist,uploader|Unknown Artist)s - %(title)s.%(ext)s"
        )

        try:
            result = await ytdlp_service.download_with_fallback_update(
                url=url,
                output_template=output_template,
                max_workers=4
            )

            all_output = result.stdout + result.stderr
            downloaded_file_path = None
            for line in all_output.split("\n"):
                if "[ExtractAudio] Destination:" in line:
                    filename_str = line.split("Destination: ")[-1].strip()
                    if filename_str:
                        output_file = Path(filename_str)
                        if (
                            output_file.is_relative_to(temp_dir)
                            and output_file.exists()
                        ):
                            downloaded_file_path = output_file
                            break
                        else:
                            logger.warning(
                                "WORKER: yt-dlp-proxy reported a file outside of temp directory: "
                                f"{filename_str}"
                            )

            if not downloaded_file_path:
                raise Exception("Downloaded file not found in temporary directory")

            music_dir = Path(settings.MUSIC_DIR_PATH)
            final_path = music_dir / downloaded_file_path.name

            shutil.move(str(downloaded_file_path), str(final_path))
            logger.info(
                f"WORKER: Moved downloaded file from {downloaded_file_path} to {final_path}"
            )

            return str(final_path)

        except Exception as e:
            logger.error(f"WORKER: Download error: {str(e)}")
            raise
