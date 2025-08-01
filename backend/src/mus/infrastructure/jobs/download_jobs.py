import asyncio
import logging
import time
from pathlib import Path

import yt_dlp

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

    ydl_opts = {
        "format": "bestaudio/best",
        "outtmpl": str(
            output_dir / "%(artist,uploader|Unknown Artist)s - %(title)s.%(ext)s"
        ),
        "extractaudio": True,
        "audioformat": "mp3",
        "audioquality": "0",
        "postprocessors": [
            {
                "key": "FFmpegExtractAudio",
                "preferredcodec": "mp3",
                "preferredquality": "0",
            },
            {
                "key": "EmbedThumbnail",
                "already_have_thumbnail": False,
            },
            {
                "key": "FFmpegMetadata",
                "add_metadata": True,
            },
        ],
        "writethumbnail": True,
        "embedthumbnail": True,
        "convertthumbnails": "jpg",
        "embedmetadata": True,
        "parse_metadata": [
            "title:%(title)s",
            "artist:%(artist,uploader|Unknown Artist)s",
        ],
        "sponsorblock_remove": [
            "sponsor",
            "selfpromo",
            "interaction",
            "intro",
            "outro",
            "preview",
            "music_offtopic",
        ],
        "sponsorblock_chapter_title": "[SponsorBlock]: %(category)s",
        "embedchapters": True,
        "concurrent_fragment_downloads": 3,
        "throttledratelimit": 100000,
        "retries": 10,
        "noplaylist": True,
        "quiet": True,
        "no_warnings": True,
    }

    with yt_dlp.YoutubeDL(ydl_opts) as ydl:
        try:
            info = ydl.extract_info(url, download=False)
            if not info:
                raise Exception("Failed to extract video information")

            title = info.get("title", "Unknown")
            logger.info(f"WORKER: Extracted info for: {title}")

            ydl.download([url])
            logger.info(f"WORKER: Download completed for: {title}")

            for file in output_dir.glob("*.mp3"):
                if file.stat().st_mtime > (time.time() - 60):
                    logger.info(f"WORKER: Found downloaded file: {file}")
                    return str(file)

            raise Exception(f"Downloaded file not found for: {title}")
        except Exception as e:
            logger.error(f"WORKER: yt-dlp error: {str(e)}")
            raise
