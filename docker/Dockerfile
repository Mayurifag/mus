FROM python:3.12-slim

WORKDIR /app

ENV PYTHONPATH=/app/src

RUN pip install uv

COPY pyproject.toml requirements.txt ./

RUN uv pip sync --system requirements.txt

COPY . .

EXPOSE 8000

CMD ["uvicorn", "src.mus.infrastructure.web.main:app", "--host", "0.0.0.0", "--port", "8000"]
