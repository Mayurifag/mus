FROM python:3.12-slim

ENV PYTHONPATH=/app

WORKDIR /app

# Install uv
RUN pip install uv

# Copy dependency files
COPY pyproject.toml requirements.txt ./

# Install dependencies
RUN uv pip sync requirements.txt

# Copy application code
COPY . .

EXPOSE 8000

CMD ["uvicorn", "src.mus.infrastructure.web.main:app", "--host", "0.0.0.0", "--port", "8000"]
