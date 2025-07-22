# Migrate Background Worker from `arq` to `streaq`

## User Problem
The application currently uses `arq` for background job processing. The user has decided to replace it with `streaq`, requiring a full migration of the background task system.

## High-Level Solution
The project will be migrated from `arq` to `streaq` for handling all background tasks. This involves updating dependencies, replacing the `arq` worker configuration with `streaq` task decorators, refactoring all job enqueuing calls to use the `streaq` broker, and updating Docker configurations to run the new worker. The existing queue priority system (`high_priority`, `medium_priority`, `low_priority`) as described in `STATE_MACHINE.md` will be implemented using named queues in `streaq`.

## Success Metrics
- The `arq` dependency is completely removed from `backend/pyproject.toml` and replaced with `streaq`.
- All background jobs (file scanning, metadata processing, track deletion, etc.) are successfully processed by the `streaq` worker.
- The application remains fully functional, and all tests pass (`make ci`).
- The `docker-compose.override.yml.example` and production `supervisord.conf` files are updated to run the `streaq` worker correctly.

## Detailed Description
The migration involves several key steps:
1.  **Dependency Management**: Swap the `arq` library for `streaq` in the project's dependencies.
2.  **Broker Configuration**: The `arq` pool manager (`arq_pool.py`) will be removed and replaced with a new module (`streaq_broker.py`) that initializes and provides a singleton `streaq` broker instance connected to DragonflyDB.
3.  **Worker and Task Refactoring**: The `arq`-specific worker configuration (`worker.py`) will be deleted. Job functions in `file_system_jobs.py` and `metadata_jobs.py` will be converted into `streaq` tasks using the `@task` decorator. The `_ctx` argument, which is specific to `arq`, will be removed from all task function signatures.
4.  **Job Enqueueing**: All application-wide calls to enqueue jobs (`arq_pool.enqueue_job(...)`) will be replaced with the `streaq` equivalent: `broker.enqueue(queue_name, task_name, **kwargs)`.
5.  **Containerization**: The `arq-worker` service in all Docker-related files (`docker-compose.yml`, `docker-compose.override.yml.example`, `supervisord.conf`) will be renamed to `streaq-worker` and its command updated to correctly launch the `streaq` worker, specifying the modules containing the tasks and the queues to monitor.

**NOTE TO ASSISTANT**: When you modify `docker-compose.override.yml.example`, you must explicitly tell the user to apply the same changes to their local `docker-compose.override.yml` file.

## Subtasks

### [ ] 1. Replace `arq` with `streaq` for background processing
**Description**: Systematically replace the `arq` background job library with `streaq`, updating all related configurations and code.
**Details**:
1.  In `backend/pyproject.toml`, remove the `arq` dependency and add `streaq>=0.1.0`.
2.  Delete the file `backend/src/mus/core/arq_pool.py`.
3.  Create a new file `backend/src/mus/core/streaq_broker.py`. In it, define and export a singleton `Broker` instance from `streaq` configured to use the `settings.DRAGONFLY_URL`.
4.  Delete the `arq`-specific worker configuration file `backend/src/mus/infrastructure/jobs/worker.py`.
5.  Refactor all job functions in `backend/src/mus/infrastructure/jobs/file_system_jobs.py` and `backend/src/mus/infrastructure/jobs/metadata_jobs.py`:
    -   Import `task` from `streaq.tasks`.
    -   Add the `@task(name="function_name")` decorator to each function intended to be a background job. Use the function name as the task name.
    -   Remove the `_ctx: Dict[str, Any]` parameter from the signature of each refactored task function.
6.  Search the codebase for all usages of `get_arq_pool` and `enqueue_job`. Replace them with calls to the new `streaq` broker.
    -   Import the broker instance from `backend.src.mus.core.streaq_broker`.
    -   Replace `arq_pool.enqueue_job("task_name", _queue_name="priority", **kwargs)` with `broker.enqueue("priority", "task_name", **kwargs)`.
    -   Ensure the queue names (`high_priority`, `medium_priority`, `low_priority`) are used correctly as per `backend/STATE_MACHINE.md`.
7.  In `docker/docker-compose.yml` and `docker/docker-compose.override.yml.example`, rename the `arq-worker` service to `streaq-worker`.
8.  Update the `command` for the new `streaq-worker` service in `docker-compose.override.yml.example` to: `sh -c "[ -f /app/requirements.txt ] && uv pip sync /app/requirements.txt; streaq worker src.mus.infrastructure.jobs.file_system_jobs src.mus.infrastructure.jobs.metadata_jobs --queue high_priority --queue medium_priority --queue low_priority"`.
9.  In `docker/production/supervisord.conf`, rename the `[program:arq-worker]` section to `[program:streaq-worker]` and update its `command` to: `streaq worker src.mus.infrastructure.jobs.file_system_jobs src.mus.infrastructure.jobs.metadata_jobs --queue high_priority --queue medium_priority --queue low_priority`.
10. Run `make back-lock` to update dependency files.
**Filepaths to Modify**: `backend/pyproject.toml`, `backend/src/mus/core/streaq_broker.py`, `backend/src/mus/infrastructure/jobs/file_system_jobs.py`, `backend/src/mus/infrastructure/jobs/metadata_jobs.py`, `backend/src/mus/infrastructure/api/routers/errors_router.py`, `backend/src/mus/infrastructure/api/routers/track_router.py`, `backend/src/mus/infrastructure/file_watcher/watcher.py`, `backend/src/mus/infrastructure/persistence/batch_operations.py`, `docker/docker-compose.yml`, `docker/docker-compose.override.yml.example`, `docker/production/supervisord.conf`
**Filepaths to Delete**: `backend/src/mus/core/arq_pool.py`, `backend/src/mus/infrastructure/jobs/worker.py`
**Relevant Make Commands**: `make back-lock`, `make ci`