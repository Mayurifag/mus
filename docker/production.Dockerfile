# Stage 1: Build frontend
FROM node:20-alpine AS frontend-builder

WORKDIR /app/frontend

# Copy frontend source code
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# Stage 2: Backend with frontend assets
FROM python:3.12-slim-bookworm

WORKDIR /app

# Install system dependencies
RUN apt-get update \
  && apt-get install -y --no-install-recommends \
  libvips-dev \
  && rm -rf /var/lib/apt/lists/*

# Install uv
RUN pip install --no-cache-dir uv

# Copy backend requirements and install dependencies
COPY backend/pyproject.toml /app/
RUN uv pip install --system /app

# Copy backend source code
COPY backend/src /app/src

# Create necessary directories
RUN mkdir -p /app/data/covers /app/music /app/static_root

# Copy frontend build from the frontend-builder stage
COPY --from=frontend-builder /app/frontend/.svelte-kit/output/client /app/static_root

# Expose the application port
EXPOSE 8000

# Set environment to production
ENV APP_ENV="production"

# Set the entrypoint
CMD ["uvicorn", "src.mus.main:app", "--host", "0.0.0.0", "--port", "8000"]
