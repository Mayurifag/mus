from arq.connections import RedisSettings

from src.mus.config import settings
from src.mus.infrastructure.jobs.metadata_jobs import process_slow_metadata
from src.mus.infrastructure.jobs.file_system_jobs import (
    handle_file_created,
    handle_file_modified,
    handle_file_deleted,
    handle_file_moved,
    delete_track_with_files,
    update_track_path_by_id,
)


class WorkerSettings:
    """ARQ worker settings with job functions and queue configuration."""

    redis_settings = RedisSettings.from_dsn(settings.DRAGONFLY_URL)

    functions = [
        # File system event handlers (use file paths)
        handle_file_created,
        handle_file_modified,
        handle_file_deleted,
        handle_file_moved,
        # ID-based job handlers (use track IDs for better serialization)
        delete_track_with_files,
        update_track_path_by_id,
        # Background processing
        process_slow_metadata,
    ]

    # Queue configuration
    queue_name = "default"

    # Worker configuration
    max_jobs = 10
    job_timeout = 300  # 5 minutes
    keep_result = 3600  # Keep results for 1 hour
