FROM python:3.12-slim

WORKDIR /app

ENV PYTHONPATH=/app/src

COPY . .

RUN pip install uv && uv pip sync --system requirements.txt

EXPOSE 8000

CMD ["uvicorn", "src.mus.infrastructure.web.main:app", "--host", "0.0.0.0", "--port", "8000"]
