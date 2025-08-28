# Replace yt-dlp Python Dependency with System-Level Nightly Binary

## User Problem
The `yt-dlp` package is currently managed as a Python dependency. This prevents the application from using the latest nightly builds, which contain important and timely fixes. The goal is to treat `yt-dlp` as a system-level tool, installed at the user level, and ensure it automatically updates to the latest nightly version.

## High-Level Solution
1.  Remove the `yt-dlp` package from the project's Python dependencies.
2.  Modify the `backend.Dockerfile` and `production.Dockerfile` to perform a user-level installation of `yt-dlp`. This involves downloading the latest stable binary to `/home/appuser/.local/bin`, making it executable, adding this directory to the user's `PATH`, and then immediately running `yt-dlp --update-to nightly` to switch to the latest nightly build.
3.  In the production environment, configure a daily cron job to run as `appuser` to keep the binary updated to the latest nightly version using the `yt-dlp --update-to nightly` command.

## Success Metrics
- The `yt-dlp` package is no longer present in `backend/pyproject.toml`.
- The `yt-dlp` command is available in the `PATH` for the `appuser` in both development and production containers.
- Executing `yt-dlp --version` inside the containers shows a nightly build identifier.
- The production container runs a daily cron job as `appuser` to update `yt-dlp`.
- The application's video download functionality remains fully operational.
- All CI checks pass.

## Detailed Description
The task is to transition `yt-dlp` from a Python package to a self-updating system binary, specifically targeting the nightly release channel. The installation will follow the user-level approach recommended in the official documentation.

**Installation Process:**
The Dockerfiles will first download the latest stable binary. Immediately after, the `--update-to nightly` command will be used to switch to and fetch the latest nightly version. This two-step process is the recommended way to get onto the nightly channel.

**Update Mechanism:**
In the production container, the `cron` daemon will be installed and configured to run as a supervised process. A cron job file will be added to execute `yt-dlp --update-to nightly` at 3:00 AM daily under the `appuser` account, ensuring permissions are handled correctly without needing root access for updates. The development environment will not have an automatic update job.

## Subtasks

### [ ] 1. Remove Python dependency and install binary with daily updates
**Description**: Update project files to remove the `yt-dlp` Python package and install it as a system binary that updates to the nightly channel daily.
**Details**:
1.  **Remove Python Dependency**:
    -   In `backend/pyproject.toml`, delete the line containing `yt-dlp>=...`.
2.  **Update Dockerfiles for Binary Installation**:
    -   In `docker/backend.Dockerfile`, before the `USER appuser` instruction, add commands to create the user-level bin directory and set its ownership: `RUN mkdir -p /home/appuser/.local/bin && chown -R appuser:appgroup /home/appuser`.
    -   In `docker/backend.Dockerfile`, after `USER appuser`, add the following:
        -   `ENV PATH="/home/appuser/.local/bin:${PATH}"`
        -   A `RUN` command that downloads the stable binary, sets permissions, and updates to nightly: `curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /home/appuser/.local/bin/yt-dlp && chmod a+rx /home/appuser/.local/bin/yt-dlp && yt-dlp --update-to nightly`.
3.  **Update Production Dockerfile and Add Cron Job**:
    -   In `docker/production/production.Dockerfile`, apply the same installation steps as for the backend Dockerfile.
    -   In the `apt-get install` section, add `cron`.
    -   Create a new file named `docker/production/yt-dlp-update.cron` with the following content: `0 3 * * * appuser /home/appuser/.local/bin/yt-dlp --update-to nightly > /dev/null 2>&1`.
    -   In `docker/production/production.Dockerfile`, add a `COPY` command to place this cron file into `/etc/cron.d/yt-dlp-update` and a `RUN` command to set its permissions (`chmod 0644 /etc/cron.d/yt-dlp-update`).
    -   In `docker/production/supervisord.conf`, add a new program for cron:
        `[program:cron]`
        `command=cron -f`
        `stdout_logfile=/dev/stdout`
        `stdout_logfile_maxbytes=0`
        `stderr_logfile=/dev/stderr`
        `stderr_logfile_maxbytes=0`
        `autorestart=true`
        `startretries=3`
**Filepaths to Modify**: `backend/pyproject.toml`, `docker/backend.Dockerfile`, `docker/production/production.Dockerfile`, `docker/production/supervisord.conf`
**Relevant Make Commands**: `make build, make ci`
