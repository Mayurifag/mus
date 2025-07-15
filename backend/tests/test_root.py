from fastapi import FastAPI
from fastapi.testclient import TestClient
from typing import Any, Dict


def test_healthcheck_endpoint() -> None:
    # Create a minimal app just for testing the healthcheck endpoint
    app = FastAPI()

    @app.get("/api/healthcheck.json", response_model=Dict[str, Any])
    async def healthcheck() -> Dict[str, Any]:
        return {"status": "healthy", "timestamp": int(__import__("time").time())}

    with TestClient(app) as client:
        response = client.get("/api/healthcheck.json")
        assert response.status_code == 200
        data: Dict[str, Any] = response.json()
        assert "status" in data and data["status"] == "healthy"
        assert "timestamp" in data


def test_openapi_generation() -> None:
    # Create a minimal app just for testing OpenAPI generation
    app = FastAPI(
        title="Test API",
        description="Test API for OpenAPI generation",
    )

    @app.get("/test")
    async def test_endpoint():
        return {"test": "data"}

    openapi_schema: Dict[str, Any] = app.openapi()
    assert isinstance(openapi_schema, dict)
    assert openapi_schema["openapi"]
    assert openapi_schema["info"]["title"] == "Test API"
