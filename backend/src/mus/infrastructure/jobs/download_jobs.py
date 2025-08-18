import asyncio
import logging
import subprocess  # nosec B404

from pathlib import Path

from src.mus.config import settings
from src.mus.core.redis import get_redis_client, set_app_write_lock
from src.mus.core.streaq_broker import worker
from src.mus.infrastructure.api.sse_handler import notify_sse_from_worker
from src.mus.infrastructure.jobs.file_system_jobs import handle_file_created

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
        output_path = await asyncio.to_thread(_download_audio, url, logger)

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


def _download_audio(url: str, logger: logging.Logger) -> str:
    output_dir = Path(settings.MUSIC_DIR_PATH)
    output_dir.mkdir(parents=True, exist_ok=True)
    output_template = str(
        output_dir / "%(artist,uploader|Unknown Artist)s - %(title)s.%(ext)s"
    )
    # yt-dlp binding not used because SponsorBlock did not work reliably
    cmd = [
        "yt-dlp",
        "--format", "bestaudio/best",
        "--extract-audio",
        "--audio-format", "mp3",
        "--audio-quality", "0",
        "-o", output_template,
        "--embed-thumbnail",
        "--convert-thumbnails", "jpg",
        "--embed-metadata",
        "--parse-metadata", "title:%(title)s",
        "--parse-metadata", "artist:%(artist,uploader|Unknown Artist)s",
        "--sponsorblock-remove", "all",
        "--embed-chapters",
        "--concurrent-fragments", "3",
        "--throttled-rate", "100K",
        "--retries", "10",
        "--no-playlist",
        url,
    ]

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)  # nosec B603

        # Parse yt-dlp output to get the actual filename
        all_output = result.stdout + result.stderr
        for line in all_output.split('\n'):
            if '[ExtractAudio] Destination:' in line:
                # Extract filename from: [ExtractAudio] Destination: /path/to/file.mp3
                filename = line.split('Destination: ')[-1].strip()
                if filename and Path(filename).exists():
                    return filename

        # # Fallback: find the most recent mp3 file
        # mp3_files = list(output_dir.glob("*.mp3"))
        # if mp3_files:
        #     latest_file = max(mp3_files, key=lambda f: f.stat().st_mtime)
        #     return str(latest_file)

        raise Exception("Downloaded file not found")

    except subprocess.CalledProcessError as e:
        logger.error(f"WORKER: yt-dlp subprocess error: {e.stderr}")
        raise Exception(f"Download failed: {e.stderr}")
    except Exception as e:
        logger.error(f"WORKER: Download error: {str(e)}")
        raise
