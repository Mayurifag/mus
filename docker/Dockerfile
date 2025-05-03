FROM python:3.12-slim

WORKDIR /app

ENV PYTHONPATH=/app/src

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    gcc \
    build-essential \
    libvips-dev \
    && rm -rf /var/lib/apt/lists/* \
    && pip install --upgrade pip \
    && pip install uv \
    ;

COPY pyproject.toml requirements.txt ./

RUN uv pip sync --system requirements.txt

COPY . .

EXPOSE 8000

CMD ["uvicorn", "src.mus.infrastructure.web.main:app", "--host", "0.0.0.0", "--port", "8000", "--proxy-headers"]
