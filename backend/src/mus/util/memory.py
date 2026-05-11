import ctypes
import ctypes.util
import gc
import logging
import os


logger = logging.getLogger(__name__)


def release_process_memory() -> None:
    try:
        import pyvips

        pyvips.vips_cache_drop_all()
    except Exception as exc:
        logger.debug("Failed to drop pyvips cache: %s", exc)

    gc.collect()

    if os.name != "posix":
        return

    try:
        libc_path = ctypes.util.find_library("c")
        if not libc_path:
            return
        malloc_trim = ctypes.CDLL(libc_path).malloc_trim
        malloc_trim(0)
    except (AttributeError, OSError):
        return
