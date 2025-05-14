import pytest
from httpx import AsyncClient, ASGITransport
from fastapi import FastAPI
import importlib
from typing import Any, Dict


@pytest.mark.asyncio
async def test_root_endpoint() -> None:
    main = importlib.import_module("mus.main")
    app: FastAPI = main.app
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as ac:
        response = await ac.get("/api")
    assert response.status_code == 200
    data: Dict[str, Any] = response.json()
    assert "status" in data and data["status"] == "ok"
    assert "message" in data


@pytest.mark.asyncio
async def test_openapi_generation() -> None:
    main = importlib.import_module("mus.main")
    app: FastAPI = main.app
    openapi_schema: Dict[str, Any] = app.openapi()
    assert isinstance(openapi_schema, dict)
    assert openapi_schema["openapi"]
    assert openapi_schema["info"]["title"]
