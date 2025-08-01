import os
import time
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.testclient import TestClient

from src.mus.infrastructure.api.routers.auth_router import router as auth_router
from src.mus.infrastructure.api.routers.player_router import router as player_router
from src.mus.infrastructure.api.routers.track_router import router as track_router


@asynccontextmanager
async def mock_lifespan(app: FastAPI):  # noqa: ARG001
    """Mock lifespan that doesn't initialize database or file watchers"""
    yield


def create_app_with_env(env_value=None):
    """Create a test app with the specified APP_ENV value"""
    if env_value is not None:
        os.environ["APP_ENV"] = env_value
    elif "APP_ENV" in os.environ:
        del os.environ["APP_ENV"]

    app = FastAPI(
        title="Mus API",
        description="Music streaming API for Mus project",
        lifespan=mock_lifespan,
    )

    # Configure CORS only if not in production
    if os.getenv("APP_ENV") != "production":
        app.add_middleware(
            CORSMiddleware,
            allow_origins=["http://localhost:5173"],
            allow_credentials=True,
            allow_methods=["*"],
            allow_headers=["*"],
        )

    # Include routers
    app.include_router(auth_router)
    app.include_router(player_router)
    app.include_router(track_router)

    @app.get("/api/healthcheck.json")
    async def healthcheck():
        return {"status": "healthy", "timestamp": int(time.time())}

    return app


def test_cors_headers_present_in_development():
    """Test that CORS headers are present when APP_ENV is 'development'"""
    app = create_app_with_env("development")
    with TestClient(app) as client:
        response = client.options(
            "/api/healthcheck.json",
            headers={
                "Origin": "http://localhost:5173",
                "Access-Control-Request-Method": "GET",
            },
        )
        assert response.status_code == 200
        assert "access-control-allow-origin" in response.headers
        assert (
            response.headers["access-control-allow-origin"] == "http://localhost:5173"
        )
        assert "access-control-allow-credentials" in response.headers
        assert response.headers["access-control-allow-credentials"] == "true"


def test_cors_headers_absent_in_production():
    """Test that CORS middleware is not applied when APP_ENV is 'production'"""
    app = create_app_with_env("production")
    with TestClient(app) as client:
        # When CORS middleware is not present, OPTIONS request will result in 405 Method Not Allowed
        # as FastAPI doesn't handle OPTIONS by default without CORS middleware
        response = client.options(
            "/api/healthcheck.json",
            headers={
                "Origin": "http://localhost:5173",
                "Access-Control-Request-Method": "GET",
            },
        )
        assert response.status_code == 405

        # Also verify that a regular GET request doesn't have CORS headers
        response = client.get(
            "/api/healthcheck.json",
            headers={"Origin": "http://localhost:5173"},
        )
        assert response.status_code == 200
        assert "access-control-allow-origin" not in response.headers


def test_cors_default_to_development_when_app_env_not_set():
    """Test that CORS headers are present when APP_ENV is not set"""
    app = create_app_with_env(None)
    with TestClient(app) as client:
        response = client.options(
            "/api/healthcheck.json",
            headers={
                "Origin": "http://localhost:5173",
                "Access-Control-Request-Method": "GET",
            },
        )
        assert response.status_code == 200
        assert "access-control-allow-origin" in response.headers
        assert (
            response.headers["access-control-allow-origin"] == "http://localhost:5173"
        )
