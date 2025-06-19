# Upgrade Backend Dependencies and Docker Workflow

## User Problem
The backend dependencies are outdated, potentially causing instability and security vulnerabilities. The development workflow is inefficient because Python packages are installed directly into the Docker image, requiring a full image rebuild for any dependency change. An unused `uv.lock` file creates confusion alongside `requirements.txt`.

## High-Level Solution
This task will perform a comprehensive upgrade of all backend Python packages and reconfigure the Docker development environment to use a persistent named volume. This will modernize the stack and significantly speed up the development cycle. The unused `backend/uv.lock` file will be removed to standardize on `requirements.txt` as the sole lock file, aligning with the project's Makefiles.

1.  **Dependency Upgrade**: All packages in `backend/pyproject.toml` will be updated to their latest stable versions. `requirements.txt` will be regenerated using `make back-lock`.
2.  **Docker Workflow Optimization**:
    *   The `docker/backend.Dockerfile` will be streamlined to create a non-root user and a Python virtual environment at `/opt/venv`, but will no longer install packages during the build.
    *   A named Docker volume, `backend_venv`, will be defined in `docker-compose.yml` and mounted in `docker-compose.override.yml.example` to persist packages.
    *   A new `make back-docker-install` command will be added to install/sync dependencies into the volume of a running container.

## Success Metrics
- All backend Python packages in `pyproject.toml` and `requirements.txt` are updated to their latest stable versions.
- The `backend/uv.lock` file is deleted.
- The backend application runs correctly with the upgraded dependencies.
- The development Docker workflow is updated:
    - Python packages are persisted in the `backend_venv` Docker volume.
    - Adding/updating packages no longer requires rebuilding the `backend` Docker image.
    - The new `make back-docker-install` command successfully installs dependencies into the running container's volume.
- All continuous integration checks (`make ci`) pass successfully after the changes.

## Detailed Description
This upgrade modernizes the backend stack and streamlines the development loop.

**Dependency Upgrade Rationale**:
The existing `pyproject.toml` contains outdated version specifiers. By updating all packages (e.g., `fastapi`, `ruff`, `pytest`, `uvicorn`), the project will benefit from the latest features, performance improvements, and security patches. The unused `backend/uv.lock` is being removed to eliminate ambiguity and rely solely on `requirements.txt` as the lock file, which is consistent with the existing `Makefile` automation.

**Docker Workflow Rationale**:
Moving package installation out of the Docker build step and into a persistent volume decouples the environment from the code and its dependencies. This allows developers to start containers once and then manage packages with a `make` command, which is significantly faster than rebuilding the image. The new `docker/backend.Dockerfile` will be leaner, focusing only on setting up the base system, a non-root user, and the virtual environment.

The new development flow will be:
1.  Run `make up` to build images and start containers.
2.  Run `make back-docker-install` once to install Python dependencies into the volume.
3.  Develop as usual. If a new Python package is needed, add it to `pyproject.toml`, run `make back-lock`, and then run `make back-docker-install`.

## Subtasks

### [ ] 1. Upgrade Dependencies and Optimize Docker Workflow
**Description**: Update all backend Python packages, update the old `uv.lock` file, and reconfigure the Docker setup to use a persistent volume for packages.
**Details**:
1.  **Update `backend/pyproject.toml`**: Modify the `[project.dependencies]` section to update all packages to their latest stable versions. Use `~=` to allow for patch and minor version updates.
2.  **Regenerate Lockfile**: Run `make back-lock` from the project root. This will use `uv pip compile` to generate an updated `backend/requirements.txt` based on the new versions in `pyproject.toml`.
4.  **Update `docker/backend.Dockerfile`**: Refactor the Dockerfile.
    - Create a non-root `appuser`.
    - Create a virtual environment at `/opt/venv`.
    - Remove the `uv pip install --system /app` command.
    - Update the `CMD` to use the `uvicorn` executable from the new virtual environment (`/opt/venv/bin/uvicorn`).
    - Ensure all files and directories have correct ownership (`appuser:appgroup`).
5.  **Update `docker-compose.yml`**: Define a new named volume called `backend_venv` under the top-level `volumes` key.
6.  **Update `docker/docker-compose.override.yml.example`**:
    - In the `backend` service, add a `volumes` entry to mount the new `backend_venv` to `/opt/venv`.
    - Update the `command` for the `backend` service to use the `uvicorn` from the virtual environment: `/opt/venv/bin/uvicorn src.mus.main:app --host 0.0.0.0 --port 8000 --reload --reload-dir src`.
    - I also think /opt/venv/bin should be in $PATH
7.  **Update `makefiles/docker.mk`**: Add a new `back-docker-install` target that executes `uv pip sync` inside the `backend` container, installing packages from `requirements.txt` into the virtual environment located at `/opt/venv`. The command should be `docker compose exec backend /opt/venv/bin/uv pip sync /app/requirements.txt`.
**Filepaths to Modify**: `backend/pyproject.toml,backend/uv.lock,docker/backend.Dockerfile,docker/docker-compose.yml,docker/docker-compose.override.yml.example,makefiles/docker.mk`
