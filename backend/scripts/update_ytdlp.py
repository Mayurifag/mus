#!/usr/bin/env python3
"""
Script to update/install yt-dlp binary and yt-dlp-proxy.
This script handles the installation and updating of both yt-dlp and yt-dlp-proxy.
"""

import asyncio
import logging
import subprocess
import sys
from pathlib import Path

logger = logging.getLogger(__name__)


class YtDlpUpdater:
    def __init__(self):
        pass

    def _run_command(self, cmd: list[str], check: bool = True, timeout: int = 300, cwd: str = None) -> subprocess.CompletedProcess:
        """Run a command with proper error handling."""
        logger.info(f"Running command: {' '.join(cmd)}")
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=check,
                timeout=timeout,
                cwd=cwd
            )
            if result.stdout:
                logger.debug(f"Command stdout: {result.stdout}")
            if result.stderr:
                logger.debug(f"Command stderr: {result.stderr}")
            return result
        except subprocess.TimeoutExpired as e:
            logger.error(f"Command timed out after {timeout}s: {' '.join(cmd)}")
            raise
        except subprocess.CalledProcessError as e:
            logger.error(f"Command failed with exit code {e.returncode}: {' '.join(cmd)}")
            logger.error(f"stderr: {e.stderr}")
            raise

    def check_yt_dlp_available(self) -> bool:
        """Check if yt-dlp is available."""
        logger.info("Checking yt-dlp availability...")

        try:
            result = self._run_command([sys.executable, "-m", "yt_dlp", "--version"])
            logger.info(f"yt-dlp version: {result.stdout.strip()}")
            return True
        except Exception as e:
            logger.error(f"yt-dlp not available: {e}")
            return False

    def install_yt_dlp_proxy(self) -> bool:
        """Install or update yt-dlp-proxy from GitHub."""
        logger.info("Installing/updating yt-dlp-proxy...")

        import tempfile
        import shutil

        try:
            with tempfile.TemporaryDirectory() as temp_dir:
                temp_path = Path(temp_dir)
                repo_path = temp_path / "yt-dlp-proxy"

                # Clone the repository
                self._run_command([
                    "git", "clone", "https://github.com/Petrprogs/yt-dlp-proxy.git",
                    str(repo_path)
                ])

                # Install dependencies
                self._run_command([
                    "uv", "pip", "install", "-r", "requirements.txt"
                ], cwd=str(repo_path))

                # Create permanent installation directory
                install_path = Path.home() / ".local" / "share" / "yt-dlp-proxy"
                if install_path.exists():
                    shutil.rmtree(install_path)
                shutil.copytree(repo_path, install_path)

                # Create wrapper script
                wrapper_path = Path.home() / ".local" / "bin" / "yt-dlp-proxy"
                wrapper_path.parent.mkdir(parents=True, exist_ok=True)

                wrapper_content = f'''#!/usr/bin/env python3
import sys
sys.path.insert(0, "{install_path}")
from main import main
if __name__ == "__main__":
    main()
'''
                wrapper_path.write_text(wrapper_content)
                wrapper_path.chmod(0o755)

                # Verify installation
                self._run_command([str(wrapper_path), "--help"])
                logger.info("yt-dlp-proxy installed successfully")

                return True
        except Exception as e:
            logger.error(f"An error occurred during yt-dlp-proxy installation: {e}", exc_info=True)
            return False

    def update_yt_dlp_proxy(self, max_workers: int = 4) -> bool:
        """Update yt-dlp-proxy with specified max workers."""
        logger.info(f"Updating yt-dlp-proxy with max-workers={max_workers}...")
        proxy_path = str(Path.home() / ".local" / "bin" / "yt-dlp-proxy")

        try:
            self._run_command([
                proxy_path, "update",
                "--max-workers", str(max_workers)
            ])
            logger.info("yt-dlp-proxy updated successfully")
            return True
        except Exception as e:
            logger.error(f"An error occurred while updating yt-dlp-proxy proxy list: {e}", exc_info=True)
            return False

    def update_all(self, max_workers: int = 4) -> bool:
        """Update yt-dlp-proxy (yt-dlp is installed during container build)."""
        logger.info("Starting update process...")

        if not self.check_yt_dlp_available():
            logger.error("yt-dlp not available - should be installed during container build")
            return False

        if not self.install_yt_dlp_proxy():
            logger.error("Failed to install yt-dlp-proxy.")
            return False

        if not self.update_yt_dlp_proxy(max_workers):
            logger.warning("Failed to update proxy list for yt-dlp-proxy. The tool is installed but may use stale proxies.")

        logger.info("Update process completed.")
        return True




async def main():
    """Main entry point for the script."""
    logging.basicConfig(
        level=logging.INFO,
        format="%(levelname)s: %(name)s - %(message)s"
    )

    updater = YtDlpUpdater()

    max_workers = 4
    if len(sys.argv) > 1:
        try:
            max_workers = int(sys.argv[1])
        except ValueError:
            logger.warning(f"Invalid max_workers value: {sys.argv[1]}, using default: 4")

    success = updater.update_all(max_workers)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    asyncio.run(main())
