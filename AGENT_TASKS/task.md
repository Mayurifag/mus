# Refactor Docker container file structure for persistent data

## User Problem
Persistent data, such as the SQLite database and generated cover images, is currently stored within the `/app` directory inside Docker containers. This approach complicates volume management, risks data loss during container rebuilds if not mounted correctly, and mixes application code with application data.

## High-Level Solution
Relocate all persistent data to a dedicated `/app_data` directory within the containers. This involves updating Dockerfiles to create the new directory structure, modifying the application configuration to use the new paths, adjusting `docker-compose` volume mounts, and updating `.dockerignore` to exclude the new data directory from image builds.

## Success Metrics
- The application starts and runs correctly using the new data directory structure.
- Docker volume mounts for persistent data correctly map a host directory (e.g., `./app_data`) to the container's `/app_data`.
- The SQLite database is created and accessed from `/app_data/database/mus.db`.
- Cover images are generated and stored in `/app_data/covers/`.
- Music files are scanned from the `/app_data/music` directory.
- Data persists on the host when containers are stopped and restarted.
- The `.dockerignore` file prevents the `app_data` directory from being included in the Docker image.
- All existing tests pass, and `make ci` completes successfully.

## Detailed Description
This task involves a structural refactoring of where the application's persistent data is stored within the Docker containers. The goal is to isolate the application's code (in `/app`) from its data (in `/app_data`).

The following changes are required:
1.  **Configuration Update:** Modify `backend/src/mus/config.py` to define and use the new `/app_data` directory for database, covers, and music storage.
2.  **Database Path Update:** Modify `backend/src/mus/database.py` to use the new database path from the configuration settings.
3.  **Startup Logic Update:** Modify `backend/src/mus/main.py` to ensure the new data directories are created on startup.
4.  **Docker Image Builds:** Update `docker/backend.Dockerfile` and `docker/production/production.Dockerfile` to create the `/app_data` directory structure and set appropriate environment variables and permissions.
5.  **Docker Compose Setup:** Update `docker/docker-compose.override.yml.example` to reflect the new volume mounting strategy. Also, apply similar changes to your local `docker-compose.override.yml` if it exists.
6.  **Ignore Files:** Update `.dockerignore`, `.gitignore`, and `backend/.dockerignore` to correctly handle the new `app_data` directory.
7.  **Cleanup:** Remove obsolete entries from `backend/.gitignore`.

## Subtasks

### [x] 1. Refactor data directories and configurations
**Description**: Move all persistent data paths to a dedicated `/app_data` directory and update all related Docker, application, and build configurations.
**Details**:
Modify the files as instructed below. If you have a local `docker-compose.override.yml`, apply the relevant changes to it as well.

-   **File**: `backend/src/mus/config.py`
    -   Remove the `BASE_DIR` constant.
    -   Remove the existing `MUSIC_DIR_PATH` and `COVERS_DIR` attributes.
    -   Add a new `DATA_DIR_PATH` attribute of type `Path`, initialized with `Path(os.getenv("DATA_DIR_PATH", "./app_data"))`.
    -   Add a new computed property `MUSIC_DIR_PATH` that returns `self.DATA_DIR_PATH / "music"`.
    -   Modify the existing `COVERS_DIR_PATH` computed property to return `self.DATA_DIR_PATH / "covers"`.
    -   Add a new computed property `DATABASE_PATH` that returns `self.DATA_DIR_PATH / "database" / "mus.db"`.

-   **File**: `backend/src/mus/database.py`
    -   Remove the hardcoded `DATABASE_FILE_PATH`.
    -   Update the `create_async_engine` call to use `settings.DATABASE_PATH` from the config, like `f"sqlite+aiosqlite:///{settings.DATABASE_PATH}"`.

-   **File**: `backend/src/mus/main.py`
    -   Inside the `lifespan` function, before `create_db_and_tables()`, add logic to create the necessary data directories.
    -   Create the database parent directory: `settings.DATABASE_PATH.parent.mkdir(parents=True, exist_ok=True)`.
    -   Create the covers directory: `settings.COVERS_DIR_PATH.mkdir(parents=True, exist_ok=True)`.
    -   Create the music directory: `settings.MUSIC_DIR_PATH.mkdir(parents=True, exist_ok=True)`.
    -   The `FileSystemScanner` should already be using `settings.MUSIC_DIR_PATH`, confirm this is correct.

-   **File**: `docker/backend.Dockerfile`
    -   Change the `ENV MUSIC_DIR_PATH` to `ENV DATA_DIR_PATH=/app_data`.
    -   Update the `RUN mkdir -p` command to create the new directory structure inside `/app_data`: `RUN mkdir -p $DATA_DIR_PATH/database $DATA_DIR_PATH/covers $DATA_DIR_PATH/music`.

-   **File**: `docker/production/production.Dockerfile`
    -   Change `ENV MUSIC_DIR_PATH=/app/music` to `ENV DATA_DIR_PATH=/app_data`.
    -   Update the `RUN mkdir -p` command to create the full `/app_data` structure: `RUN mkdir -p /app_data/database /app_data/covers /app_data/music /var/log/supervisor`.
    -   Update the `chown` command to include `/app_data`: `chown -R appuser:appgroup /app_data /app/frontend/build`.

-   **File**: `docker/docker-compose.override.yml.example`
    -   In the `backend` service's volumes, change `- ../data:/app/data` to `- ../app_data:/app_data`.
    -   Remove the volume mount for music: `- /path/to/your/music:/app/music`. Add a comment indicating that music should now be placed in `./app_data/music` on the host.
    -   Update the environment variable comments to reflect that `DATA_DIR_PATH` is the primary variable and `MUSIC_DIR_PATH` is derived from it.

-   **File**: `.dockerignore`
    -   Remove `**/mus_database.db`.
    -   Add `app_data/` to the common ignore section.

-   **File**: `.gitignore`
    -   Remove `mus_database.db`, `/music`, and `/data`.
    -   Add `/app_data` to the "Application specific" section.

-   **File**: `backend/.dockerignore`
    -   Remove `*.db`, `mus_database.db`, `/music`, and `/data/`.
    -   Add `/app_data/` to the "Data directories" section.

-   **File**: `backend/.gitignore`
    -   Remove all content from this file, as its rules are now covered by the root `.gitignore`.

**Filepaths to Modify**: `backend/src/mus/config.py,backend/src/mus/database.py,backend/src/mus/main.py,docker/backend.Dockerfile,docker/production/production.Dockerfile,docker/docker-compose.override.yml.example,.dockerignore,.gitignore,backend/.dockerignore,backend/.gitignore`
**Relevant Make Commands (Optional)**: `make build, make up, make back-test`
