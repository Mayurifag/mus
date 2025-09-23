FROM node:24-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/ ./
ENV VITE_INTERNAL_API_HOST="http://127.0.0.1:8001"
ENV VITE_PUBLIC_API_HOST=""
RUN npm ci --no-fund --prefer-offline \
    && npm run build \
    ;

FROM ghcr.io/astral-sh/uv:python3.13-bookworm-slim AS backend-builder

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        gcc \
        libvips-dev

ENV PYTHONUNBUFFERED=1 \
    VIRTUAL_ENV=/opt/venv \
    UV_PROJECT_ENVIRONMENT=/opt/venv \
    UV_LINK_MODE=copy \
    UV_COMPILE_BYTECODE=1 \
    UV_PYTHON_DOWNLOADS=never
ENV PATH="/opt/venv/bin:$PATH"

WORKDIR /app/backend

COPY backend/pyproject.toml backend/uv.lock backend/README.md* ./
COPY backend/src ./src

RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --locked --no-dev --no-editable

FROM python:3.13-slim-bookworm
ARG TARGETARCH
ARG BUILD_DATE
ARG COMMIT_SHA
ARG COMMIT_TITLE

ENV PYTHONUNBUFFERED=1 \
    APP_ENV=production \
    LOG_LEVEL=info \
    DATA_DIR_PATH=/app_data \
    BACKEND_URL=http://127.0.0.1:8001 \
    REDIS_URL=redis://127.0.0.1:6379 \
    VIRTUAL_ENV=/opt/venv \
    NODE_VERSION=24 \
    WATCHFILES_FORCE_POLLING=true \
    WATCHFILES_POLL_DELAY_MS=2000 \
    PATH="/opt/venv/bin:$PATH"

LABEL org.opencontainers.image.created=$BUILD_DATE
LABEL org.opencontainers.image.revision=$COMMIT_SHA
LABEL org.opencontainers.image.title=$COMMIT_TITLE

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        nginx \
        supervisor \
        libnginx-mod-http-brotli-filter \
        libnginx-mod-http-brotli-static \
        libvips42 \
        curl \
        gettext-base \
        ffmpeg \
        redis-server \
        sqlite3 \
        cron \
    && curl -fsSL https://deb.nodesource.com/setup_${NODE_VERSION}.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

ARG UID=1000
ARG GID=1000
RUN groupadd --gid ${GID} appgroup && \
    useradd --uid ${UID} --gid ${GID} --shell /bin/bash --create-home appuser && \
    mkdir -p /home/appuser/.local/bin && \
    chown -R appuser:appgroup /home/appuser

WORKDIR /app

COPY --from=backend-builder /opt/venv /opt/venv
COPY --chown=appuser:appgroup backend/src /app/src

COPY --from=frontend-builder --chown=appuser:appgroup /app/frontend/build /app/frontend/build
COPY --from=frontend-builder --chown=appuser:appgroup /app/frontend/node_modules /app/frontend/node_modules
COPY --from=frontend-builder --chown=appuser:appgroup /app/frontend/package.json /app/frontend/package.json

COPY docker/production/login.html /app/docker/production/login.html
COPY docker/production/nginx.conf.template /app/docker/production/nginx.conf.template
COPY docker/production/start.sh /app/start.sh
COPY docker/production/supervisord.conf /etc/supervisor/conf.d/supervisord.conf
COPY docker/production/yt-dlp-update.cron /etc/cron.d/yt-dlp-update

ENV PATH="/home/appuser/.local/bin:${PATH}"

RUN chmod +x /app/start.sh && \
    chmod 0644 /etc/cron.d/yt-dlp-update && \
    mkdir -p /app_data/database /app_data/covers /app_data/music /var/log/supervisor /var/cache/nginx/covers_cache && \
    chown -R appuser:appgroup /app_data /app/frontend/build && \
    chown appuser:appgroup /app && \
    chmod 666 /etc/nginx/nginx.conf || touch /etc/nginx/nginx.conf && chmod 666 /etc/nginx/nginx.conf

USER appuser
RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /home/appuser/.local/bin/yt-dlp && \
    chmod a+rx /home/appuser/.local/bin/yt-dlp && \
    yt-dlp --update-to nightly

USER root

EXPOSE 8000

HEALTHCHECK --interval=300s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8000/api/healthcheck.json || exit 1

CMD ["/app/start.sh"]
