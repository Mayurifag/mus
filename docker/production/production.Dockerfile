FROM node:24-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/ ./
ENV VITE_INTERNAL_API_HOST="http://127.0.0.1:8001"
ENV VITE_PUBLIC_API_HOST=""
RUN npm ci --no-fund --prefer-offline \
    && npm run build \
    ;

FROM ghcr.io/astral-sh/uv:python3.13-bookworm-slim AS backend-builder
ENV PYTHONUNBUFFERED=1
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        gcc \
        libvips-dev
WORKDIR /app/backend_build_temp
RUN uv venv /opt/venv
ENV VIRTUAL_ENV=/opt/venv
ENV PATH="/opt/venv/bin:$PATH"
COPY backend/pyproject.toml backend/README.md* ./
COPY backend/src ./src
RUN uv pip install --no-cache .

FROM python:3.13-slim-bookworm
ARG TARGETARCH
ENV PYTHONUNBUFFERED=1 \
    APP_ENV=production \
    LOG_LEVEL=info \
    DATA_DIR_PATH=/app_data \
    BACKEND_URL=http://127.0.0.1:8001 \
    VIRTUAL_ENV=/opt/venv \
    DRAGONFLY_VERSION=1.31.1 \
    NODE_VERSION=24 \
    PATH="/opt/venv/bin:$PATH"

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        nginx \
        supervisor \
        libnginx-mod-http-brotli-filter \
        libnginx-mod-http-brotli-static \
        libvips42 \
        curl \
        gettext-base \
        ffmpeg \
    && ARCHITECTURE="x86_64" && \
    if [ "$TARGETARCH" = "arm64" ]; then ARCHITECTURE="aarch64"; fi && \
    mkdir -p /tmp/dragonfly && \
    curl -fL "https://github.com/dragonflydb/dragonfly/releases/download/v${DRAGONFLY_VERSION}/dragonfly-${ARCHITECTURE}.tar.gz" -o /tmp/dragonfly/dragonfly.tar.gz && \
    cd /tmp/dragonfly && \
    tar -xzf dragonfly.tar.gz && \
    mv "dragonfly-${ARCHITECTURE}" /usr/local/bin/dragonfly && \
    chmod +x /usr/local/bin/dragonfly && \
    cd / && \
    rm -rf /tmp/dragonfly && \
    curl -fsSL https://deb.nodesource.com/setup_${NODE_VERSION}.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

ARG UID=10001
ARG GID=10001
RUN groupadd --gid ${GID} appgroup && \
    useradd --uid ${UID} --gid ${GID} --shell /bin/bash --create-home appuser

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

RUN chmod +x /app/start.sh && \
    mkdir -p /app_data/database /app_data/covers /app_data/music /var/log/supervisor && \
    chown -R appuser:appgroup /app_data /app/frontend/build && \
    chown appuser:appgroup /app

EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
  CMD curl -f http://localhost:8000/api/healthcheck.json && \
      curl -f http://localhost:8000/ -o /dev/null || exit 1

CMD ["/app/start.sh"]
