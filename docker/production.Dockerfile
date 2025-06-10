FROM node:20-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci --no-fund --prefer-offline
COPY frontend/ ./
RUN npm run build

#################################################################

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
    mv /root/.cargo/bin/uv /usr/local/bin/uv && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app/backend_build_temp
COPY backend/pyproject.toml backend/README.md* ./
COPY backend/src ./src
RUN uv pip install --system --no-cache .

#################################################################

FROM python:3.12-slim-bookworm
ENV PYTHONUNBUFFERED=1 \
    APP_ENV=production \
    LOG_LEVEL=info
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
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
COPY --from=backend-builder /usr/local/bin/gunicorn /usr/local/bin/gunicorn
COPY --chown=appuser:appgroup backend/src /app/src
COPY --from=frontend-builder --chown=appuser:appgroup /app/frontend/build /app/frontend/build
RUN mkdir -p /app/data/covers /app/music && \
    chown -R appuser:appgroup /app/data /app/music /app/frontend/build && \
    chown appuser:appgroup /app
USER appuser
EXPOSE 8000
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8000/api || exit 1
CMD ["uvicorn", "src.mus.main:app", "--host", "0.0.0.0", "--port", "8000"]
