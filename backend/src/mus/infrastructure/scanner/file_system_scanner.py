import os
import asyncio
from collections.abc import AsyncGenerator
from pathlib import Path
from typing import ClassVar, List, Optional, Set
import logging

logger = logging.getLogger(__name__)


class FileSystemScanner:
    SUPPORTED_EXTENSIONS: ClassVar[set[str]] = {".mp3", ".flac"}

    def __init__(self):
        self.default_music_dir_str = os.getenv("MUSIC_DIR", "./music")
        self.default_root_dir = Path(self.default_music_dir_str)

    async def _recursive_scan_single_directory(
        self,
        directory: Path,
        extensions: Set[str],
        min_mtime: Optional[int] = None,
    ) -> AsyncGenerator[Path, None]:
        try:
            if not await asyncio.to_thread(directory.is_dir):
                return
        except OSError as e:
            logger.warning(f"Cannot access directory {directory}: {e}")
            return

        try:
            dir_contents_iter = await asyncio.to_thread(directory.iterdir)
            dir_contents = await asyncio.to_thread(list, dir_contents_iter)
        except OSError as e:
            logger.warning(f"Cannot read directory contents {directory}: {e}")
            return

        for item in dir_contents:
            try:
                if await asyncio.to_thread(item.is_file):
                    if item.suffix.lower() in extensions:
                        if min_mtime is not None:
                            item_stat = await asyncio.to_thread(item.stat)
                            file_mtime = int(item_stat.st_mtime)
                            if file_mtime > min_mtime:
                                yield item
                        else:
                            yield item
            except OSError as e:
                logger.warning(f"Cannot access file item {item}: {e}")

        for item in dir_contents:
            try:
                if await asyncio.to_thread(item.is_dir):
                    async for music_file in self._recursive_scan_single_directory(
                        item, extensions, min_mtime
                    ):
                        yield music_file
            except OSError as e:
                logger.warning(
                    f"Cannot access subdirectory item {item} for recursion: {e}"
                )

    async def scan_directories(
        self,
        directories: Optional[List[Path]] = None,
        extensions: Optional[Set[str]] = None,
        min_mtime: Optional[int] = None,
    ) -> AsyncGenerator[Path, None]:
        scan_dirs_resolved: List[Path]
        if directories:
            scan_dirs_resolved = directories
        else:
            try:
                await asyncio.to_thread(
                    self.default_root_dir.mkdir, parents=True, exist_ok=True
                )
                scan_dirs_resolved = [self.default_root_dir]
            except OSError as e:
                logger.error(
                    f"Failed to create or access default music directory {self.default_root_dir}: {e}"
                )
                return

        effective_extensions = extensions or self.SUPPORTED_EXTENSIONS

        for base_dir in scan_dirs_resolved:
            if not await asyncio.to_thread(base_dir.exists):
                logger.warning(
                    f"Provided scan directory {base_dir} does not exist. Skipping."
                )
                continue
            if not await asyncio.to_thread(base_dir.is_dir):
                logger.warning(
                    f"Provided scan path {base_dir} is not a directory. Skipping."
                )
                continue

            async for music_file in self._recursive_scan_single_directory(
                base_dir, effective_extensions, min_mtime
            ):
                yield music_file
