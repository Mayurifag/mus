FROM ghcr.io/astral-sh/uv:python3.13-bookworm-slim

WORKDIR /app

ENV DATA_DIR_PATH=/app_data
ENV VIRTUAL_ENV=/opt/venv
ENV PATH="/opt/venv/bin:$PATH"

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        gcc \
        libvips-dev \
        ffmpeg \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r appgroup && useradd -r -g appgroup --create-home appuser \
    && mkdir -p $DATA_DIR_PATH/database $DATA_DIR_PATH/covers $DATA_DIR_PATH/music \
    && uv venv /opt/venv \
    && chown -R appuser:appgroup /app /opt/venv $DATA_DIR_PATH /home/appuser

USER appuser

EXPOSE 8000

CMD ["sh", "-c", "[ -f /app/requirements.txt ] && uv pip sync /app/requirements.txt; uvicorn src.mus.main:app --host 0.0.0.0 --port 8000 --reload --reload-delay 0.5"]
