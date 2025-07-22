from arq.connections import RedisSettings

from src.mus.config import settings
from src.mus.infrastructure.jobs.metadata_jobs import process_slow_metadata
from src.mus.infrastructure.jobs.file_system_jobs import (
    handle_file_created,
    handle_file_modified,
    handle_file_deleted,
    handle_file_moved,
    delete_track_with_files,
)


class WorkerSettings:
    """ARQ worker settings with job functions and queue configuration."""

    redis_settings = RedisSettings.from_dsn(settings.DRAGONFLY_URL)

    functions = [
        # High priority jobs (user actions, critical file events)
        handle_file_created,
        handle_file_deleted,
        delete_track_with_files,
        # Medium priority jobs (file updates)
        handle_file_modified,
        handle_file_moved,
        # Low priority jobs (background processing)
        process_slow_metadata,
    ]

    # Queue configuration
    queue_name = "default"

    # Worker configuration
    max_jobs = 10
    job_timeout = 300  # 5 minutes
    keep_result = 3600  # Keep results for 1 hour
