import pytest
import os
from pathlib import Path
from unittest.mock import patch, AsyncMock, MagicMock, PropertyMock
from fastapi.testclient import TestClient
from sqlalchemy.ext.asyncio import AsyncEngine
from src.mus.main import app as actual_app
from src.mus.config import Config


async def immediate_run(coro):
    await coro
    return MagicMock()


@pytest.mark.asyncio
async def test_startup_process_runs_unconditionally():
    mock_engine_begin_conn_ctx_mgr = AsyncMock()
    mock_engine_begin_conn_ctx_mgr.__aenter__.return_value = (
        mock_engine_begin_conn_ctx_mgr
    )

    async def fake_run_sync(func_to_run, *args, **kwargs):
        if callable(func_to_run):
            return func_to_run(*args, **kwargs)
        return None

    mock_engine_begin_conn_ctx_mgr.run_sync = AsyncMock(side_effect=fake_run_sync)

    mock_engine_begin_method = MagicMock(return_value=mock_engine_begin_conn_ctx_mgr)

    mock_engine_instance = MagicMock(spec=AsyncEngine)
    mock_engine_instance.begin = mock_engine_begin_method

    mock_settings_covers_dir = MagicMock(spec=Path)

    with (
        patch("src.mus.infrastructure.database.engine", new=mock_engine_instance),
        patch(
            "src.mus.infrastructure.database.SQLModel.metadata.create_all"
        ) as mock_sql_create_all,
        patch.object(
            Config,
            "COVERS_DIR_PATH",
            new_callable=PropertyMock,
            return_value=mock_settings_covers_dir,
        ),
    ):
        original_app_env = os.environ.get("APP_ENV")
        with patch("src.mus.main.settings.APP_ENV", "test"):
            with TestClient(actual_app) as client:
                client.get("/api/healthcheck.json")

                assert mock_engine_begin_method.call_count == 1
                assert mock_sql_create_all.call_count == 1

                mock_settings_covers_dir.mkdir.assert_called_once_with(
                    parents=True, exist_ok=True
                )

        if original_app_env is None:
            os.environ.pop("APP_ENV", None)
        else:
            os.environ["APP_ENV"] = original_app_env


@pytest.mark.asyncio
async def test_startup_process_runs_in_production():
    original_app_env = os.environ.get("APP_ENV")
    with patch("src.mus.main.settings.APP_ENV", "production"):
        mock_scanner = AsyncMock()
        mock_scanner.scan.return_value = mock_scanner

        with patch(
            "src.mus.main.InitialScanner.create_default", return_value=mock_scanner
        ):
            with TestClient(actual_app) as client:
                client.get("/api/healthcheck.json")

                mock_scanner.scan.assert_awaited_once()

    if original_app_env is not None:
        os.environ["APP_ENV"] = original_app_env
    elif "APP_ENV" in os.environ:
        del os.environ["APP_ENV"]
