import asyncio
import logging
import time
from contextlib import asynccontextmanager
from typing import Any, Dict

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from src.mus.application.services.permissions_service import PermissionsService
from src.mus.application.use_cases.fast_initial_scan import FastInitialScanUseCase
from src.mus.application.use_cases.slow_initial_scan import SlowInitialScanUseCase
from src.mus.config import settings
from src.mus.infrastructure.api.routers import (
    auth_router,
    errors_router,
    permissions_router,
    player_router,
    track_router,
)
from src.mus.infrastructure.api.sse_handler import router as sse_router
from src.mus.infrastructure.database import create_db_and_tables
from src.mus.infrastructure.file_watcher.watcher import watch_music_directory

logging.basicConfig(
    level=settings.LOG_LEVEL.upper(), format="%(levelname)s:     %(name)s - %(message)s"
)
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(_: FastAPI):
    await create_db_and_tables()

    permissions_service = PermissionsService()
    await asyncio.to_thread(permissions_service.check_write_permissions)

    fast_scanner = await FastInitialScanUseCase.create_default()
    await fast_scanner.execute()

    async def run_slow_scan_with_logging():
        """Wrapper to ensure slow scan exceptions are logged and don't get lost."""
        try:
            logger.info("Starting async slow metadata scan task...")
            slow_scanner = SlowInitialScanUseCase()
            await (
                slow_scanner.execute()
            )  # This runs async but the task itself is fire-and-forget
            logger.info("Async slow metadata scan task completed successfully")
        except Exception as e:
            logger.critical(
                f"CRITICAL: Async slow metadata scan task failed with exception: {e}",
                exc_info=True,
            )

    # Start slow scan as async fire-and-forget task (as intended)
    # The task runs async, but we don't await it here so startup isn't blocked
    slow_scan_task = asyncio.create_task(run_slow_scan_with_logging())

    # Add callback to log final task status
    def log_slow_scan_completion(task):
        if task.cancelled():
            logger.info("Slow scan task was cancelled")
        elif task.exception():
            logger.error(f"Slow scan task failed with exception: {task.exception()}")
        else:
            logger.info("Slow scan task completed successfully")

    slow_scan_task.add_done_callback(log_slow_scan_completion)
    watcher_task = asyncio.create_task(watch_music_directory())

    try:
        yield
    finally:
        logger.info("Application shutting down, cancelling background tasks...")
        watcher_task.cancel()
        slow_scan_task.cancel()

        try:
            await watcher_task
        except asyncio.CancelledError:
            logger.info("File watcher task cancelled successfully")
        except Exception as e:
            logger.error(
                f"Error while cancelling file watcher task: {e}", exc_info=True
            )

        try:
            await slow_scan_task
        except asyncio.CancelledError:
            logger.info("Slow scan task cancelled successfully")
        except Exception as e:
            logger.error(f"Error while cancelling slow scan task: {e}", exc_info=True)


app = FastAPI(
    title="mus backend",
    description="FastAPI python backend for mus project",
    lifespan=lifespan,
)

if settings.APP_ENV != "production":
    app.add_middleware(
        CORSMiddleware,
        allow_origins=[settings.FRONTEND_ORIGIN],
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

app.include_router(auth_router.router)
app.include_router(player_router.router)
app.include_router(track_router.router)
app.include_router(permissions_router.router)
app.include_router(errors_router.router)
app.include_router(sse_router)


@app.get("/api/healthcheck.json", response_model=Dict[str, Any])
async def healthcheck() -> Dict[str, Any]:
    return {"status": "healthy", "timestamp": int(time.time())}
