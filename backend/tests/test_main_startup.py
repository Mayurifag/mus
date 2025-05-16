import pytest
import os
from pathlib import Path
from unittest.mock import patch, AsyncMock, MagicMock
from fastapi.testclient import TestClient
from src.mus.main import app as actual_app


async def immediate_run(coro):
    await coro
    return MagicMock()


@pytest.mark.asyncio
async def test_startup_process_runs_unconditionally():
    mock_engine_begin_conn = AsyncMock()
    mock_engine_begin_conn.__aenter__.return_value = mock_engine_begin_conn
    mock_engine_begin_conn.run_sync = AsyncMock()
    mock_actual_engine_begin = MagicMock(return_value=mock_engine_begin_conn)
    with patch(
        "sqlalchemy.ext.asyncio.AsyncEngine.begin", new=mock_actual_engine_begin
    ), patch("src.mus.main.SQLModel.metadata.drop_all") as mock_sql_drop_all, patch(
        "src.mus.main.SQLModel.metadata.create_all"
    ) as mock_sql_create_all, patch(
        "src.mus.main.shutil.rmtree"
    ) as mock_shutil_rmtree, patch(
        "src.mus.main.os.makedirs"
    ) as mock_os_makedirs, patch("src.mus.main.Path") as mock_path_constructor, patch(
        "src.mus.main.asyncio.create_task", new=MagicMock()
    ) as mock_asyncio_create_task:
        mock_covers_dir_path_instance = MagicMock(spec=Path)
        mock_path_constructor.return_value = mock_covers_dir_path_instance
        with TestClient(actual_app) as client:
            client.get("/api")
            mock_actual_engine_begin.assert_called_once()
            mock_engine_begin_conn.run_sync.assert_any_call(mock_sql_drop_all)
            mock_engine_begin_conn.run_sync.assert_any_call(mock_sql_create_all)
            mock_path_constructor.assert_called_with("./data/covers")
            mock_shutil_rmtree.assert_called_once_with(
                mock_covers_dir_path_instance, ignore_errors=True
            )
            mock_os_makedirs.assert_called_once_with(
                mock_covers_dir_path_instance, exist_ok=True
            )
            mock_asyncio_create_task.assert_called_once()


@pytest.mark.asyncio
async def test_startup_process_runs_in_production():
    os.environ["APP_ENV"] = "production"
    mock_engine_begin_conn = AsyncMock()
    mock_engine_begin_conn.__aenter__.return_value = mock_engine_begin_conn
    mock_engine_begin_conn.run_sync = AsyncMock()
    mock_actual_engine_begin = MagicMock(return_value=mock_engine_begin_conn)
    with patch(
        "sqlalchemy.ext.asyncio.AsyncEngine.begin", new=mock_actual_engine_begin
    ), patch("src.mus.main.SQLModel.metadata.drop_all") as mock_sql_drop_all, patch(
        "src.mus.main.SQLModel.metadata.create_all"
    ) as mock_sql_create_all, patch(
        "src.mus.main.shutil.rmtree"
    ) as mock_shutil_rmtree, patch(
        "src.mus.main.os.makedirs"
    ) as mock_os_makedirs, patch("src.mus.main.Path") as mock_path_constructor, patch(
        "src.mus.main.asyncio.create_task", new=MagicMock()
    ) as mock_asyncio_create_task:
        mock_covers_dir_path_instance = MagicMock(spec=Path)
        mock_path_constructor.return_value = mock_covers_dir_path_instance
        with TestClient(actual_app) as client:
            client.get("/api")
            mock_actual_engine_begin.assert_called_once()
            mock_engine_begin_conn.run_sync.assert_any_call(mock_sql_drop_all)
            mock_engine_begin_conn.run_sync.assert_any_call(mock_sql_create_all)
            mock_path_constructor.assert_called_with("./data/covers")
            mock_shutil_rmtree.assert_called_once_with(
                mock_covers_dir_path_instance, ignore_errors=True
            )
            mock_os_makedirs.assert_called_once_with(
                mock_covers_dir_path_instance, exist_ok=True
            )
            mock_asyncio_create_task.assert_called_once()
    del os.environ["APP_ENV"]
