import pytest
import asyncio
import logging
from unittest.mock import AsyncMock, MagicMock

from src.mus.infrastructure.tasks.background_scanner import PeriodicScanner
from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase
from src.mus.application.dtos.scan import ScanResponseDTO

logger = logging.getLogger("src.mus.infrastructure.tasks.background_scanner")


@pytest.fixture
def mock_scan_use_case() -> MagicMock:
    use_case = MagicMock(spec=ScanTracksUseCase)
    use_case.scan_directory = AsyncMock(
        return_value=ScanResponseDTO(
            success=True,
            message="Scan complete",
            tracks_changes=1,
            errors=0,
        )
    )
    return use_case


@pytest.fixture
def periodic_scanner(mock_scan_use_case: MagicMock) -> PeriodicScanner:
    return PeriodicScanner(
        scan_use_case=mock_scan_use_case,
        scan_interval_seconds=1,
    )


@pytest.mark.asyncio
async def test_periodic_scanner_start_and_stop(
    periodic_scanner: PeriodicScanner,
    mock_scan_use_case: MagicMock,
):
    await periodic_scanner.start()
    assert periodic_scanner._task is not None
    await asyncio.sleep(0.1)
    await periodic_scanner.stop()

    mock_scan_use_case.scan_directory.assert_awaited()


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
    caplog,
):
    mock_scan_use_case.scan_directory.side_effect = Exception("Scan failed!")

    # Start and quickly stop to test error handling
    await periodic_scanner.start()
    await asyncio.sleep(0.1)
    await periodic_scanner.stop()

    assert any("Scan error: Scan failed!" in message for message in caplog.messages)
