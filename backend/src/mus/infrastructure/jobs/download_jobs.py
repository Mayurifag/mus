import asyncio
import logging
import os
import shutil
import tempfile
from pathlib import Path
from urllib.parse import urlparse

from mutagen.easyid3 import EasyID3
from mutagen.id3 import ID3NoHeaderError

from src.mus.config import settings
from src.mus.core.redis import get_redis_client, set_app_write_lock
from src.mus.core.streaq_broker import worker
from src.mus.domain.services.progress_parser import parse_progress_line
from src.mus.infrastructure.api.sse_handler import notify_sse_from_worker
from src.mus.infrastructure.jobs.file_system_jobs import handle_file_created


def _validate_url(url: str) -> bool:
    try:
        parsed = urlparse(url)
        return parsed.scheme in ["http", "https"] and bool(parsed.netloc)
    except Exception:
        return False


@worker.task()
async def download_track_from_url(
    url: str, title: str | None = None, artist: str | None = None
):
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
        with tempfile.TemporaryDirectory(prefix="mus-download-") as tmp:
            temp_dir = Path(tmp)
            if title and artist:
                safe = f"{artist.replace('/', '_')} - {title.replace('/', '_')}"
                tpl = str(temp_dir / f"{safe}.%(ext)s")
            else:
                tpl = str(
                    temp_dir / "%(artist,uploader|Unknown Artist)s - %(title)s.%(ext)s"
                )
            cache = str(settings.DATA_DIR_PATH / ".cache")
            pm_artist = "artist:%(artist,uploader|Unknown Artist)s"
            cmd = [
                "yt-dlp",
                "--format",
                "bestaudio/best",
                "--extract-audio",
                "--audio-format",
                "mp3",
                "--audio-quality",
                "0",
                "-o",
                tpl,
                "--embed-thumbnail",
                "--convert-thumbnails",
                "jpg",
                "--embed-metadata",
                "--parse-metadata",
                "title:%(title)s",
                "--parse-metadata",
                pm_artist,
                "--sponsorblock-remove",
                "all",
                "--embed-chapters",
                "--concurrent-fragments",
                "3",
                "--throttled-rate",
                "100K",
                "--retries",
                "10",
                "--no-playlist",
                "--cache-dir",
                cache,
                "--newline",
                "--progress",
            ]
            if (
                settings.COOKIES_FILE_PATH.exists()
                and settings.COOKIES_FILE_PATH.is_file()
            ):
                cmd.extend(["--cookies", str(settings.COOKIES_FILE_PATH)])
            cmd.append(url)
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.STDOUT,
            )
            if proc.stdout is None:
                raise RuntimeError("stdout pipe not created")
            output_lines: list[str] = []
            async for raw in proc.stdout:
                line = raw.decode().strip()
                output_lines.append(line)
                progress = parse_progress_line(line)
                if progress is not None:
                    await notify_sse_from_worker(
                        action_key="download_progress", payload=progress
                    )
            logger.info(f"WORKER: yt-dlp produced {len(output_lines)} output lines")
            await asyncio.wait_for(proc.wait(), timeout=600)
            if proc.returncode != 0:
                tail = "\n".join(output_lines[-20:])
                raise Exception(f"Download failed (exit {proc.returncode}): {tail}")
            dl_path = None
            for line in output_lines:
                if "[ExtractAudio] Destination:" in line:
                    f = Path(line.split("Destination: ")[-1].strip())
                    if f.is_relative_to(temp_dir) and f.exists():
                        dl_path = f
                        break
                    logger.warning(f"WORKER: file outside temp dir: {f}")
            if not dl_path:
                raise Exception("Downloaded file not found in temporary directory")
            final_path = Path(settings.MUSIC_DIR_PATH) / dl_path.name
            shutil.move(str(dl_path), str(final_path))
            os.chmod(str(final_path), 0o666)  # nosec B103
            logger.info(f"WORKER: Moved {dl_path} to {final_path}")
            if title and artist and final_path.suffix.lower() == ".mp3":
                try:
                    audio = EasyID3(str(final_path))
                except ID3NoHeaderError:
                    audio = EasyID3()
                    audio.filename = str(final_path)
                audio["title"] = title
                audio["artist"] = artist
                audio.save(str(final_path))
            output_path = str(final_path)
        await set_app_write_lock(output_path)
        async with worker:
            await handle_file_created.enqueue(
                file_path_str=output_path, skip_slow_metadata=False
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
            await client.delete("download_lock:global")
            logger.info("WORKER: Released download lock")
        finally:
            await client.aclose()
    logger.info(f"WORKER: Completed download for URL: {url}")
