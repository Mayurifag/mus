# Upgrade Python and Node.js Versions

## User Problem
The project is pinned to older versions of Python (3.12) and Node.js (20), which may lack the latest features, performance improvements, and security patches. This also creates a maintenance burden by not staying current with the technology stack.

## High-Level Solution
Systematically update all references to Python and Node.js versions across the project's configuration files, Dockerfiles, and CI/CD workflows to their latest stable releases. Subsequently, regenerate all dependency lock files to ensure compatibility with the new runtime versions.

## Success Metrics
- All Python version specifiers are updated to the latest stable version.
- All Node.js version specifiers are updated to the latest stable version.
- The `backend/uv.lock` file is removed to standardize on `requirements.txt`.
- `backend/requirements.txt` and `frontend/package-lock.json` are successfully regenerated.
- The application builds and runs correctly in both development (Docker Compose) and production (Docker) environments.
- All continuous integration checks (`make ci`) pass without errors.

## Detailed Description
This task involves a comprehensive version bump for the core runtimes. The AI assistant must identify the latest stable versions of Python and Node.js and apply them consistently across the project.

**Python Upgrade Path:**
The target Python version must be updated in `pyproject.toml`, all Dockerfiles that use a Python base image (`docker/backend.Dockerfile`, `docker/production/production.Dockerfile`), and all GitHub Actions workflows that set up a Python environment (`.github/workflows/linters.yml`). After updating versions, the `requirements.txt` lockfile must be regenerated using the `make back-lock` command. The `backend/uv.lock` file must be deleted to eliminate ambiguity and standardize on a single lock file format.

**Node.js Upgrade Path:**
The target Node.js version must be updated in all relevant Dockerfiles (`docker/frontend.Dockerfile`, `docker/production/production.Dockerfile`) and GitHub Actions workflows (`.github/workflows/linters.yml`). An `engines` block must be added to `frontend/package.json` to enforce the new version constraint. The `frontend/package-lock.json` file must be updated by running `make front-install` to reflect any dependency changes.

## Subtasks

### [ ] 1. Upgrade Python and Node.js versions and dependencies
**Description**: Update all Python and Node.js version specifiers across the project and regenerate all associated dependency lockfiles.
**Details**:
1.  **Python Version Upgrade**:
    *   Determine the latest stable Python version (e.g., 3.13).
    *   In `backend/pyproject.toml`, update the `requires-python` constraint.
    *   In `.github/workflows/linters.yml`, update the `python-version` in both the `backend-lint` and `backend-test` jobs.
    *   In `docker/backend.Dockerfile`, update the Python version in the `FROM` instruction to a corresponding `uv` image tag.
    *   In `docker/production/production.Dockerfile`, update the Python versions for both the `backend-builder` stage and the final `python:...-slim-bookworm` stage.
2.  **Node.js Version Upgrade**:
    *   Determine the latest stable Node.js LTS version (e.g., 24).
    *   In `.github/workflows/linters.yml`, update the `node-version` in both the `frontend-lint` and `frontend-test` jobs.
    *   In `docker/frontend.Dockerfile`, update the Node.js version in the `FROM` instruction.
    *   In `docker/production/production.Dockerfile`, update the Node.js version for the `frontend-builder` stage.
    *   In `frontend/package.json`, add an `engines` block to specify the new Node.js version constraint.
3.  **Dependency Regeneration**:
    *   Delete the `backend/uv.lock` file.
    *   Run `make back-lock` to regenerate `backend/requirements.txt` for the new Python version.
    *   Run `make front-install` to update `frontend/package-lock.json` for the new Node.js environment.

4. Check project for previous versions, because you might have to update some old files not listed in task yet.
**Filepaths to Modify**: `backend/pyproject.toml,.github/workflows/linters.yml,docker/backend.Dockerfile,docker/frontend.Dockerfile,docker/production/production.Dockerfile,frontend/package.json,backend/uv.lock`
**Relevant Make Commands (Optional)**: `make back-lock, make front-install, make ci`
