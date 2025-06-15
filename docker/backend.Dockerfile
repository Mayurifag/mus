FROM python:3.12-slim-bookworm

WORKDIR /app

ENV DATA_DIR_PATH=/app_data

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
  gcc \
  libvips-dev \
  && rm -rf /var/lib/apt/lists/*

# Install uv
RUN pip install --no-cache-dir uv

COPY backend/ /app/

RUN uv pip install --system /app

RUN mkdir -p $DATA_DIR_PATH/database $DATA_DIR_PATH/covers $DATA_DIR_PATH/music

# Expose the application port
EXPOSE 8000

# Set the entrypoint
CMD ["uvicorn", "src.mus.main:app", "--host", "0.0.0.0", "--port", "8000"]
