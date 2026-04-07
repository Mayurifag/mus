import tempfile
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from src.mus.infrastructure.jobs.download_jobs import download_track_from_url


def _make_mock_proc(stdout_lines: list[bytes], returncode: int = 0):
    """Build a mock asyncio.Process with async-iterable stdout."""

    class AsyncIterLines:
        def __init__(self, lines):
            self._lines = iter(lines)

        def __aiter__(self):
            return self

        async def __anext__(self):
            try:
                return next(self._lines)
            except StopIteration:
                raise StopAsyncIteration

    proc = MagicMock()
    proc.stdout = AsyncIterLines(stdout_lines)
    proc.returncode = returncode
    proc.wait = AsyncMock(return_value=returncode)
    return proc


@pytest.mark.asyncio
async def test_download_command_includes_cache_dir():
    """Download command must include --cache-dir flag pointing at .cache."""
    proc = _make_mock_proc([b"[ExtractAudio] Destination: /fake/Artist - Track.mp3"])
    with (
        patch(
            "asyncio.create_subprocess_exec",
            new_callable=AsyncMock,
            return_value=proc,
        ) as mock_exec,
        patch(
            "src.mus.infrastructure.jobs.download_jobs.notify_sse_from_worker",
            new_callable=AsyncMock,
        ),
        patch(
            "src.mus.infrastructure.jobs.download_jobs.set_app_write_lock",
            new_callable=AsyncMock,
        ),
        patch(
            "src.mus.infrastructure.jobs.download_jobs.handle_file_created"
        ) as mock_hfc,
        patch(
            "src.mus.infrastructure.jobs.download_jobs.get_redis_client"
        ) as mock_redis,
        patch("shutil.move"),
        patch("os.chmod"),
        patch("pathlib.Path.exists", return_value=True),
        patch("pathlib.Path.is_relative_to", return_value=True),
    ):
        mock_redis_client = AsyncMock()
        mock_redis.return_value = mock_redis_client
        mock_hfc.enqueue = AsyncMock()

        try:
            await download_track_from_url.fn(
                url="https://example.com/video", title="Track", artist="Artist"
            )
        except Exception:
            pass

        mock_exec.assert_called_once()
        cmd = mock_exec.call_args[0]
        assert "--cache-dir" in cmd
        idx = cmd.index("--cache-dir")
        assert cmd[idx + 1].endswith("/.cache")


@pytest.mark.asyncio
async def test_download_command_includes_cookies_when_file_exists():
    """Download command must include --cookies when cookies file exists."""
    proc = _make_mock_proc([b"[ExtractAudio] Destination: /fake/Artist - Track.mp3"])
    with tempfile.TemporaryDirectory() as tmp:
        cookies_file = Path(tmp) / "cookies.txt"
        cookies_file.write_text("# cookies")

        with (
            patch(
                "asyncio.create_subprocess_exec",
                new_callable=AsyncMock,
                return_value=proc,
            ) as mock_exec,
            patch(
                "src.mus.infrastructure.jobs.download_jobs.settings"
            ) as mock_settings,
            patch(
                "src.mus.infrastructure.jobs.download_jobs.notify_sse_from_worker",
                new_callable=AsyncMock,
            ),
            patch(
                "src.mus.infrastructure.jobs.download_jobs.set_app_write_lock",
                new_callable=AsyncMock,
            ),
            patch(
                "src.mus.infrastructure.jobs.download_jobs.handle_file_created"
            ) as mock_hfc,
            patch(
                "src.mus.infrastructure.jobs.download_jobs.get_redis_client"
            ) as mock_redis,
            patch("shutil.move"),
            patch("os.chmod"),
            patch("pathlib.Path.exists", return_value=True),
            patch("pathlib.Path.is_relative_to", return_value=True),
        ):
            mock_settings.COOKIES_FILE_PATH = cookies_file
            mock_settings.DATA_DIR_PATH = Path(tmp)
            mock_settings.MUSIC_DIR_PATH = Path(tmp)
            mock_redis_client = AsyncMock()
            mock_redis.return_value = mock_redis_client
            mock_hfc.enqueue = AsyncMock()

            try:
                await download_track_from_url.fn(
                    url="https://example.com/video", title="Track", artist="Artist"
                )
            except Exception:
                pass

            mock_exec.assert_called_once()
            cmd = mock_exec.call_args[0]
            assert "--cookies" in cmd
            idx = cmd.index("--cookies")
            assert cmd[idx + 1] == str(cookies_file)


@pytest.mark.asyncio
async def test_download_command_excludes_cookies_when_file_missing():
    """Download command must not include --cookies when cookies file is absent."""
    proc = _make_mock_proc([b"[ExtractAudio] Destination: /fake/Artist - Track.mp3"])
    with tempfile.TemporaryDirectory() as tmp:
        missing_cookies = Path(tmp) / "no_cookies.txt"

        with (
            patch(
                "asyncio.create_subprocess_exec",
                new_callable=AsyncMock,
                return_value=proc,
            ) as mock_exec,
            patch(
                "src.mus.infrastructure.jobs.download_jobs.settings"
            ) as mock_settings,
            patch(
                "src.mus.infrastructure.jobs.download_jobs.notify_sse_from_worker",
                new_callable=AsyncMock,
            ),
            patch(
                "src.mus.infrastructure.jobs.download_jobs.set_app_write_lock",
                new_callable=AsyncMock,
            ),
            patch(
                "src.mus.infrastructure.jobs.download_jobs.handle_file_created"
            ) as mock_hfc,
            patch(
                "src.mus.infrastructure.jobs.download_jobs.get_redis_client"
            ) as mock_redis,
            patch("shutil.move"),
            patch("os.chmod"),
            patch("pathlib.Path.exists", return_value=True),
            patch("pathlib.Path.is_relative_to", return_value=True),
        ):
            mock_settings.COOKIES_FILE_PATH = missing_cookies
            mock_settings.DATA_DIR_PATH = Path(tmp)
            mock_settings.MUSIC_DIR_PATH = Path(tmp)
            mock_redis_client = AsyncMock()
            mock_redis.return_value = mock_redis_client
            mock_hfc.enqueue = AsyncMock()

            try:
                await download_track_from_url.fn(
                    url="https://example.com/video", title="Track", artist="Artist"
                )
            except Exception:
                pass

            mock_exec.assert_called_once()
            cmd = mock_exec.call_args[0]
            assert "--cookies" not in cmd


@pytest.mark.asyncio
async def test_download_progress_emits_sse_events():
    """Progress lines must trigger download_progress SSE events."""
    progress_lines = [
        b"[download]   25.0% of ~10.00MiB at  1.00MiB/s ETA 00:08",
        b"[download]   50.0% of ~10.00MiB at  2.00MiB/s ETA 00:04",
        b"[info] Some info line",
        b"[download] 100% of   10.00MiB in 00:00:05",
        b"[ExtractAudio] Destination: /fake/Artist - Track.mp3",
    ]
    proc = _make_mock_proc(progress_lines)

    with (
        patch(
            "asyncio.create_subprocess_exec",
            new_callable=AsyncMock,
            return_value=proc,
        ),
        patch(
            "src.mus.infrastructure.jobs.download_jobs.notify_sse_from_worker",
            new_callable=AsyncMock,
        ) as mock_notify,
        patch(
            "src.mus.infrastructure.jobs.download_jobs.set_app_write_lock",
            new_callable=AsyncMock,
        ),
        patch(
            "src.mus.infrastructure.jobs.download_jobs.handle_file_created"
        ) as mock_hfc,
        patch(
            "src.mus.infrastructure.jobs.download_jobs.get_redis_client"
        ) as mock_redis,
        patch("shutil.move"),
        patch("os.chmod"),
        patch("pathlib.Path.exists", return_value=True),
        patch("pathlib.Path.is_relative_to", return_value=True),
    ):
        mock_redis_client = AsyncMock()
        mock_redis.return_value = mock_redis_client
        mock_hfc.enqueue = AsyncMock()

        try:
            await download_track_from_url.fn(
                url="https://example.com/video", title="Track", artist="Artist"
            )
        except Exception:
            pass

    progress_calls = [
        call
        for call in mock_notify.call_args_list
        if call.kwargs.get("action_key") == "download_progress"
    ]
    assert len(progress_calls) == 3  # 25%, 50%, 100%
    assert progress_calls[0].kwargs["payload"] == {
        "percent": 25.0,
        "speed": "1.00MiB/s",
        "eta": "00:08",
    }
    assert progress_calls[1].kwargs["payload"] == {
        "percent": 50.0,
        "speed": "2.00MiB/s",
        "eta": "00:04",
    }
    assert progress_calls[2].kwargs["payload"] == {
        "percent": 100.0,
        "speed": "",
        "eta": "",
    }
