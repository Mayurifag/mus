FROM python:3.12-slim-bookworm

WORKDIR /app

# Install system dependencies
RUN apt-get update \
  && apt-get install -y --no-install-recommends \
  libvips-dev \
  && rm -rf /var/lib/apt/lists/*

# Install uv
RUN pip install --no-cache-dir uv

# Copy requirements and install dependencies
COPY backend/pyproject.toml /app/
RUN uv pip install --system /app

# Create necessary directories
RUN mkdir -p /app/data/covers /app/music

# Copy application code
COPY backend/src /app/src

# Expose the application port
EXPOSE 8000

# Set the entrypoint
CMD ["uvicorn", "src.mus.main:app", "--host", "0.0.0.0", "--port", "8000"]
