# Align Container Permissions with Host Volume

## User Problem
When a music directory is mounted from the host, file permission errors occur inside the container if the host user's UID/GID does not match the container's `appuser` UID/GID. This prevents the application from reading or writing to the music library in both development and production environments.

## High-Level Solution
Create a single, unified entrypoint script for both development and production containers that runs as root. This script will detect the UID/GID of the mounted `/app_data/music` directory and dynamically adjust the container's internal `appuser` and `appgroup` to match. It will then execute the appropriate command for the environment (development server or production supervisor). This ensures seamless file access across all environments without requiring the user to manually manage permissions.

## Success Metrics
- The application can read and write to the mounted `/app_data/music` directory regardless of the host user's UID/GID in both development (`make up`) and production (`make test-prod`) environments.
- File operations (e.g., editing tags, deleting files, processing covers) succeed without permission errors.
- The containers start correctly.
- `make ci` passes.

## Detailed Description
A new, unified script named `docker/entrypoint.sh` will be created to serve as the single source of truth for container startup logic.

**Synchronization Logic:**
1.  **Ownership Detection**: Use `stat -c '%u'` and `stat -c '%g'` on `/app_data/music` to get its owner UID and GID from the host.
2.  **ID Comparison**: Compare these detected values with the current UID and GID of the `appuser` inside the container.
3.  **Dynamic ID Adjustment**: If the IDs differ, execute `groupmod` and `usermod` to update the container's `appgroup` and `appuser` to match.
4.  **Ownership Correction**: After adjustment, `chown` other critical directories (`/app_data/database`, `/app_data/covers`, `/home/appuser`) to ensure they are owned by the correctly configured `appuser`.

**Environment-Specific Execution:**
The script will use the `APP_ENV` environment variable to determine the final execution step:
-   If `APP_ENV` is "production", it will perform the Nginx configuration substitution and then `exec supervisord` to start all production services as root (supervisord will then drop privileges for individual services as configured).
-   For any other `APP_ENV` (e.g., "development"), it will switch to the `appuser` and execute the command passed to the container (e.g., `uvicorn` or `streaq`).

This approach centralizes the permission logic, reduces code duplication, and provides a consistent startup process for all environments.

## Subtasks

### [ ] 1. Create unified entrypoint script and update Dockerfiles
**Description**: Create a single `entrypoint.sh` for both development and production to handle permission alignment, and update all relevant Dockerfiles to use it.
**Details**:
1.  Create a new executable file `docker/entrypoint.sh`.
2.  Implement the permission synchronization logic inside this script. It should check if `/app_data/music` exists, then use `stat` to get its UID/GID, compare with `appuser`'s UID/GID, and use `groupmod`/`usermod` if they differ. Finally, `chown` the necessary directories.
3.  Add conditional logic at the end of the script:
    - If `$APP_ENV` is `production`, perform the `envsubst` for Nginx and then `exec /usr/bin/supervisord`.
    - Otherwise, execute `exec su - appuser -c "$*"`.
4.  Modify `docker/backend.Dockerfile` to:
    - `COPY` the new `docker/entrypoint.sh` script.
    - Make it executable.
    - Remove the `USER appuser` line.
    - Set `ENTRYPOINT ["entrypoint.sh"]`.
5.  Modify `docker/production/production.Dockerfile` to:
    - Replace the `COPY` command for `start.sh` with one for `entrypoint.sh`.
    - Change the `CMD` to `["/app/entrypoint.sh"]`.
6.  Delete the old `docker/production/start.sh` file.
**Filepaths to Modify**: `docker/backend.Dockerfile`, `docker/production/production.Dockerfile`, `docker/production/start.sh`, `docker/entrypoint.sh`
**Relevant Make Commands (Optional)**: `make up, make test-prod`