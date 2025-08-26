"""
Service for managing yt-dlp-proxy integration and updates.
"""

import asyncio
import logging
import subprocess
import sys
from pathlib import Path
from typing import Optional

logger = logging.getLogger(__name__)


class YtDlpService:
    """Service for managing yt-dlp-proxy operations."""

    def __init__(self):
        self.update_script_path = Path(__file__).parent.parent.parent.parent.parent / "scripts" / "update_ytdlp.py"
        self._update_lock = asyncio.Lock()
        self._last_update_attempt: Optional[float] = None
        self._update_cooldown = 300  # 5 minutes cooldown between update attempts

    async def run_update_script(self, max_workers: int = 4) -> bool:
        """
        Run the yt-dlp update script.

        Args:
            max_workers: Maximum number of workers for yt-dlp-proxy update

        Returns:
            True if update was successful, False otherwise
        """
        import time

        async with self._update_lock:
            # Check cooldown
            current_time = time.time()
            if (self._last_update_attempt and
                current_time - self._last_update_attempt < self._update_cooldown):
                logger.info("Update script called too recently, skipping")
                return False

            self._last_update_attempt = current_time

            logger.info(f"Running yt-dlp update script with max_workers={max_workers}")

            try:
                result = await asyncio.to_thread(
                    subprocess.run,
                    [sys.executable, str(self.update_script_path), str(max_workers)],
                    capture_output=True,
                    text=True,
                    timeout=600  # 10 minutes timeout
                )

                if result.returncode == 0:
                    logger.info("yt-dlp update script completed successfully")
                    if result.stdout:
                        logger.debug(f"Update script stdout: {result.stdout}")
                    return True
                else:
                    logger.error(f"yt-dlp update script failed with exit code {result.returncode}")
                    if result.stderr:
                        logger.error(f"Update script stderr: {result.stderr}")
                    return False

            except subprocess.TimeoutExpired:
                logger.error("yt-dlp update script timed out")
                return False
            except Exception as e:
                logger.error(f"Failed to run yt-dlp update script: {e}")
                return False

    async def download_with_proxy(
        self,
        url: str,
        output_template: str,
        additional_args: Optional[list[str]] = None
    ) -> subprocess.CompletedProcess:
        """
        Download using yt-dlp-proxy instead of direct yt-dlp.

        Args:
            url: URL to download
            output_template: Output filename template
            additional_args: Additional arguments for yt-dlp

        Returns:
            CompletedProcess result

        Raises:
            subprocess.CalledProcessError: If download fails
            subprocess.TimeoutExpired: If download times out
        """
        base_args = [
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
        ]

        if additional_args:
            base_args.extend(additional_args)

        base_args.append(url)

        # Use yt-dlp-proxy from expected location
        proxy_path = str(Path.home() / ".local" / "bin" / "yt-dlp-proxy")
        cmd = [proxy_path] + base_args

        logger.info(f"Running yt-dlp-proxy command: {' '.join(cmd[:5])}... (truncated)")

        try:
            result = await asyncio.to_thread(
                subprocess.run,
                cmd,
                capture_output=True,
                text=True,
                check=True,
                timeout=600  # 10 minutes timeout
            )

            logger.info("yt-dlp-proxy download completed successfully")
            return result

        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as e:
            logger.error(f"yt-dlp-proxy download failed: {e}")
            raise

    async def download_with_fallback_update(
        self,
        url: str,
        output_template: str,
        additional_args: Optional[list[str]] = None,
        max_workers: int = 4
    ) -> subprocess.CompletedProcess:
        """
        Download with automatic fallback to update if download fails.

        Args:
            url: URL to download
            output_template: Output filename template
            additional_args: Additional arguments for yt-dlp
            max_workers: Maximum workers for update if needed

        Returns:
            CompletedProcess result

        Raises:
            Exception: If download fails even after update attempt
        """
        try:
            # First attempt
            return await self.download_with_proxy(url, output_template, additional_args)

        except Exception as first_error:
            logger.warning(f"First download attempt failed: {first_error}")
            logger.info("Attempting to update yt-dlp and yt-dlp-proxy...")

            # Try to update
            update_success = await self.run_update_script(max_workers)

            if not update_success:
                logger.error("Update failed, re-raising original download error")
                raise first_error

            logger.info("Update completed, retrying download...")

            try:
                # Second attempt after update
                return await self.download_with_proxy(url, output_template, additional_args)

            except Exception as second_error:
                logger.error(f"Download failed even after update: {second_error}")
                raise second_error

    async def ensure_ytdlp_proxy_available(self) -> bool:
        """
        Ensure yt-dlp-proxy is available and working.

        Returns:
            True if yt-dlp-proxy is available, False otherwise
        """
        proxy_path = str(Path.home() / ".local" / "bin" / "yt-dlp-proxy")

        try:
            result = await asyncio.to_thread(
                subprocess.run,
                [proxy_path, "--help"],
                capture_output=True,
                text=True,
                timeout=30
            )

            if result.returncode == 0:
                logger.info("yt-dlp-proxy is available")
                return True
            else:
                logger.warning("yt-dlp-proxy not working properly")
                return False

        except Exception as e:
            logger.warning(f"yt-dlp-proxy not available: {e}")
            return False


# Global service instance
ytdlp_service = YtDlpService()
