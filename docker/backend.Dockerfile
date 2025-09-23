FROM ghcr.io/astral-sh/uv:python3.13-bookworm-slim

WORKDIR /app

ENV DATA_DIR_PATH=/app_data
ENV PATH="/opt/venv/bin:$PATH"
ENV VIRTUAL_ENV=/opt/venv
ENV UV_PROJECT_ENVIRONMENT=/opt/venv
ENV RUFF_CACHE_DIR=/tmp/.ruff_cache
ENV XDG_CACHE_HOME=/app_data/.cache
ENV HOME=/home/appuser

ARG USER_ID=1000
ARG GROUP_ID=1000

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
        gcc \
        libvips-dev \
        ffmpeg \
        curl \
        sudo \
    && rm -rf /var/lib/apt/lists/*

RUN (groupadd -g $GROUP_ID appgroup || true) \
    && useradd -u $USER_ID -g $(getent group $GROUP_ID | cut -d: -f1) --create-home appuser \
    && mkdir -p $DATA_DIR_PATH/database $DATA_DIR_PATH/covers $DATA_DIR_PATH/music $DATA_DIR_PATH/.cache /opt/venv /home/appuser/.local/bin \
    && chown -R appuser:$(getent group $GROUP_ID | cut -d: -f1) /app $DATA_DIR_PATH /home/appuser /opt/venv \
    && echo "appuser ALL=(ALL) NOPASSWD: /bin/chown -R appuser* /opt/venv*" >> /etc/sudoers

USER appuser

ENV PATH="/home/appuser/.local/bin:${PATH}"

RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /home/appuser/.local/bin/yt-dlp && \
    chmod a+rx /home/appuser/.local/bin/yt-dlp && \
    yt-dlp --update-to nightly

RUN uv venv /opt/venv

EXPOSE 8001

CMD ["sh", "-c", "sudo chown -R appuser:$(id -gn appuser) /opt/venv 2>/dev/null || true; uv sync --locked --no-editable; uvicorn src.mus.main:app --host 0.0.0.0 --port 8001 --reload --timeout-graceful-shutdown 1"]
