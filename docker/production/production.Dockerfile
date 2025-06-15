FROM node:20-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci --no-fund --prefer-offline
COPY frontend/ ./
RUN npm run build

FROM python:3.12-slim-bookworm AS backend-builder
ENV PYTHONUNBUFFERED=1 \
    PIP_NO_CACHE_DIR=off \
    UV_SYSTEM_PYTHON=1
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    gcc \
    libvips-dev \
    curl \
    && \
    curl -LsSf https://astral.sh/uv/install.sh | sh && \
    mv /root/.local/bin/uv /usr/local/bin/uv && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app/backend_build_temp
COPY backend/pyproject.toml backend/README.md* ./
COPY backend/src ./src
RUN uv pip install --system --no-cache .

FROM python:3.12-slim-bookworm
ENV PYTHONUNBUFFERED=1 \
    APP_ENV=production \
    LOG_LEVEL=info \
    DATA_DIR_PATH=/app_data

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    nginx \
    supervisor \
    nodejs \
    libnginx-mod-http-brotli-filter \
    libnginx-mod-http-brotli-static \
    libvips42 \
    curl \
    && rm -rf /var/lib/apt/lists/*

ARG UID=10001
ARG GID=10001
RUN groupadd --gid ${GID} appgroup && \
    useradd --uid ${UID} --gid ${GID} --shell /bin/bash --create-home appuser

WORKDIR /app

COPY --from=backend-builder /usr/local/lib/python3.12/site-packages /usr/local/lib/python3.12/site-packages
COPY --from=backend-builder /usr/local/bin/uvicorn /usr/local/bin/uvicorn
COPY --chown=appuser:appgroup backend/src /app/src
COPY --from=frontend-builder --chown=appuser:appgroup /app/frontend/build /app/frontend/build

# SvelteKit adapter-node generates ES modules but doesn't create package.json
# Node.js requires "type": "module" to import ES modules properly
RUN echo '{"type": "module"}' > /app/frontend/build/package.json && \
    chown appuser:appgroup /app/frontend/build/package.json

COPY docker/production/nginx.conf /etc/nginx/nginx.conf
COPY docker/production/supervisord.conf /etc/supervisor/conf.d/supervisord.conf

RUN mkdir -p /app_data/database /app_data/covers /app_data/music /var/log/supervisor && \
    chown -R appuser:appgroup /app_data /app/frontend/build && \
    chown appuser:appgroup /app

EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8000/api || exit 1

CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf"]
