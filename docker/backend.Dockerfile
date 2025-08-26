FROM ghcr.io/astral-sh/uv:python3.13-bookworm-slim

WORKDIR /app

ENV DATA_DIR_PATH=/app_data
ENV PATH="/opt/venv/bin:$PATH"
ENV UV_COMPILE_BYTECODE=1
ENV UV_LINK_MODE=copy
ENV VIRTUAL_ENV=/opt/venv
ENV UV_PROJECT_ENVIRONMENT=/opt/venv
ENV RUFF_CACHE_DIR=/tmp/.ruff_cache

ARG USER_ID=1000
ARG GROUP_ID=1000

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
        gcc \
        libvips-dev \
        ffmpeg \
        curl \
        git \
    && rm -rf /var/lib/apt/lists/*

RUN (groupadd -g $GROUP_ID appgroup || true) \
    && useradd -u $USER_ID -g $(getent group $GROUP_ID | cut -d: -f1) --create-home appuser \
    && mkdir -p $DATA_DIR_PATH/database $DATA_DIR_PATH/covers $DATA_DIR_PATH/music /opt/venv \
    && chown -R appuser:$(getent group $GROUP_ID | cut -d: -f1) /app $DATA_DIR_PATH /home/appuser /opt/venv

USER appuser

RUN uv venv /opt/venv

EXPOSE 8001

CMD ["sh", "-c", "uv sync --all-extras; uv pip install -U --pre 'yt-dlp[default]'; python scripts/update_ytdlp.py 4 || true; uvicorn src.mus.main:app --host 0.0.0.0 --port 8001 --reload --timeout-graceful-shutdown 1"]
