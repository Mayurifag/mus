import pytest
import asyncio
import logging
from unittest.mock import AsyncMock, MagicMock, patch

from src.mus.infrastructure.tasks.background_scanner import PeriodicScanner
from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase
from src.mus.application.dtos.scan import ScanResponseDTO

logger = logging.getLogger("src.mus.infrastructure.tasks.background_scanner")


@pytest.fixture
def mock_broadcast_func() -> AsyncMock:
    return AsyncMock()


@pytest.fixture
def mock_scan_use_case() -> MagicMock:
    use_case = MagicMock(spec=ScanTracksUseCase)
    use_case.scan_directory = AsyncMock(
        return_value=ScanResponseDTO(
            success=True,
            message="Scan complete",
            tracks_added=1,
            tracks_updated=0,
            errors=0,
        )
    )
    return use_case


@pytest.fixture
def periodic_scanner(
    mock_scan_use_case: MagicMock, mock_broadcast_func: AsyncMock
) -> PeriodicScanner:
    return PeriodicScanner(
        scan_use_case=mock_scan_use_case,
        scan_interval_seconds=1,
        broadcast_func=mock_broadcast_func,
    )


@pytest.mark.asyncio
async def test_periodic_scanner_start_and_stop(
    periodic_scanner: PeriodicScanner,
    mock_scan_use_case: MagicMock,
    mock_broadcast_func: AsyncMock,
):
    expected_scan_response = ScanResponseDTO(
        success=True,
        message="Scan complete",
        tracks_added=1,
        tracks_updated=0,
        errors=0,
    )
    mock_scan_use_case.scan_directory.return_value = expected_scan_response

    call_count = 0

    async def mock_wait_for_next_scan() -> bool:
        nonlocal call_count
        call_count += 1
        return False if call_count == 1 else True

    periodic_scanner._wait_for_next_scan = mock_wait_for_next_scan

    await periodic_scanner.start()
    assert periodic_scanner._task is not None
    await asyncio.sleep(0.2)
    await periodic_scanner.stop()

    mock_scan_use_case.scan_directory.assert_awaited_once()
    mock_broadcast_func.assert_called_once_with(
        {
            "type": "scan_completed",
            "summary": expected_scan_response.model_dump(),
        }
    )


@pytest.mark.asyncio
async def test_periodic_scanner_start_idempotency(periodic_scanner: PeriodicScanner):
    periodic_scanner._run_periodic_scan = AsyncMock()

    await periodic_scanner.start()
    first_task = periodic_scanner._task
    assert first_task is not None

    await periodic_scanner.start()
    assert periodic_scanner._task is first_task

    await periodic_scanner.stop()
    assert periodic_scanner._task is None


@pytest.mark.asyncio
async def test_periodic_scanner_stop_when_not_running(
    periodic_scanner: PeriodicScanner,
):
    periodic_scanner._task = None
    await periodic_scanner.stop()


@pytest.mark.asyncio
async def test_periodic_scanner_handles_scan_exception(
    periodic_scanner: PeriodicScanner,
    mock_scan_use_case: MagicMock,
    mock_broadcast_func: AsyncMock,
    caplog,
):
    expected_successful_scan_response = ScanResponseDTO(
        success=True, message="Scan OK", tracks_added=1, tracks_updated=0, errors=0
    )
    mock_scan_use_case.scan_directory.side_effect = [
        Exception("Scan failed!"),
        expected_successful_scan_response,
    ]

    result1 = await periodic_scanner._perform_single_scan()
    assert result1 is None

    result2 = await periodic_scanner._perform_single_scan()
    assert result2 == expected_successful_scan_response

    assert mock_scan_use_case.scan_directory.call_count == 2
    assert any(
        "Error during scan directory operation: Scan failed!" in message
        for message in caplog.messages
    )

    mock_broadcast_func.assert_called_once_with(
        {
            "type": "scan_completed",
            "summary": expected_successful_scan_response.model_dump(),
        }
    )


@pytest.mark.asyncio
async def test_wait_for_next_scan_timeout(periodic_scanner: PeriodicScanner):
    with patch("asyncio.wait_for", side_effect=asyncio.TimeoutError):
        result = await periodic_scanner._wait_for_next_scan()
        assert result is True


@pytest.mark.asyncio
async def test_wait_for_next_scan_stop_event(periodic_scanner: PeriodicScanner):
    periodic_scanner._stop_event.set()
    result = await periodic_scanner._wait_for_next_scan()
    assert result is False


@pytest.mark.asyncio
async def test_run_periodic_scan_full_cycle(
    periodic_scanner: PeriodicScanner,
    mock_scan_use_case: MagicMock,
    mock_broadcast_func: AsyncMock,
):
    call_count = 0

    async def mock_wait_for_next_scan() -> bool:
        nonlocal call_count
        call_count += 1
        return call_count < 2

    periodic_scanner._wait_for_next_scan = mock_wait_for_next_scan
    await periodic_scanner._run_periodic_scan()

    assert mock_scan_use_case.scan_directory.call_count == 2
    assert mock_broadcast_func.call_count == 2
