import logging
from streaq import Worker

from src.mus.config import settings

# Configure logging for the worker
logging.basicConfig(
    level=settings.LOG_LEVEL.upper(), format="%(levelname)s:     %(name)s - %(message)s"
)

worker = Worker(redis_url=settings.REDIS_URL)

# Import task modules to register tasks with the worker
import src.mus.infrastructure.jobs.file_system_jobs  # noqa: F401, E402
import src.mus.infrastructure.jobs.metadata_jobs  # noqa: F401, E402
