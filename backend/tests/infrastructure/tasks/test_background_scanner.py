import pytest
import asyncio
from unittest.mock import AsyncMock, MagicMock, patch

from src.mus.infrastructure.tasks.background_scanner import PeriodicScanner
from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase
from src.mus.application.dtos.scan import ScanResponseDTO

# Store original asyncio.wait_for
original_asyncio_wait_for = asyncio.wait_for


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
def periodic_scanner_fixture(mock_scan_use_case: MagicMock) -> PeriodicScanner:
    # Use a very short interval for realistic loop progression in tests
    return PeriodicScanner(scan_use_case=mock_scan_use_case, scan_interval_seconds=1)


@pytest.mark.asyncio
async def test_periodic_scanner_start_and_stop(
    periodic_scanner_fixture: PeriodicScanner, mock_scan_use_case: MagicMock
):
    scanner = periodic_scanner_fixture
    # Configure the mock use case to return a specific ScanResponseDTO
    expected_scan_response = ScanResponseDTO(
        success=True,
        message="Scan complete",
        tracks_added=1,
        tracks_updated=0,
        errors=0,
    )
    mock_scan_use_case.scan_directory.return_value = expected_scan_response

    # Define side_effect function locally to capture scanner and manage its own counter for this test
    async def custom_wait_for_side_effect(awaitable, timeout):
        if not hasattr(custom_wait_for_side_effect, "calls_in_loop"):
            custom_wait_for_side_effect.calls_in_loop = 0

        if timeout == scanner._scan_interval:  # Call from the loop's wait for interval
            custom_wait_for_side_effect.calls_in_loop += 1
            if custom_wait_for_side_effect.calls_in_loop == 1:
                scanner._stop_event.set()  # Set stop event before raising Timeout to ensure loop terminates after this scan
                raise asyncio.TimeoutError()  # Trigger first (and only) scan
            # This path (calls_in_loop > 1 for loop wait) should ideally not be hit if stop_event is effective.
            # If it is, let original wait_for handle the already set stop_event.
            return await original_asyncio_wait_for(awaitable, timeout=0.001)
        else:  # Call from scanner.stop() waiting for the task, or other calls
            return await original_asyncio_wait_for(awaitable, timeout=timeout)

    with patch(
        "src.mus.infrastructure.tasks.background_scanner.broadcast_sse_event",
        new_callable=AsyncMock,
    ) as mock_broadcast_sse_module_level, patch(
        "asyncio.wait_for", side_effect=custom_wait_for_side_effect
    ) as mock_wait_for_patch_obj:  # Use the mock obj if needed
        _ = mock_wait_for_patch_obj  # Explicitly use to satisfy linter if F841 occurs
        scanner._broadcast_sse_event = (
            mock_broadcast_sse_module_level  # Make the instance use the mock
        )

        await scanner.start()
        assert scanner._task is not None

        await asyncio.sleep(
            0.1
        )  # Allow time for one scan cycle to start & hit the mocked wait_for

        await scanner.stop()

        mock_scan_use_case.scan_directory.assert_awaited_once()
        mock_broadcast_sse_module_level.assert_any_call(
            {
                "type": "scan_completed",
                "summary": expected_scan_response.model_dump(),
            }
        )


@pytest.mark.asyncio
async def test_periodic_scanner_start_idempotency(
    periodic_scanner_fixture: PeriodicScanner,
):
    scanner = periodic_scanner_fixture
    # Also mock the _run_periodic_scan to prevent unawaited coroutine warnings
    scanner._run_periodic_scan = AsyncMock()

    with patch("asyncio.create_task") as mock_create_task:
        mock_task_instance = AsyncMock(
            spec=asyncio.Task
        )  # Use AsyncMock for awaitable task
        # asyncio.Task.done() is a regular method, not async
        mock_task_instance.done = MagicMock(return_value=False)
        mock_create_task.return_value = mock_task_instance

        await scanner.start()
        assert scanner._task == mock_task_instance
        mock_create_task.assert_called_once()

        await scanner.start()
        mock_create_task.assert_called_once()
        assert scanner._task == mock_task_instance

        # Cleanup
        scanner._stop_event.set()
        mock_task_instance.done.return_value = True  # Now task is done
        if scanner._task and not scanner._task.done():
            try:
                await original_asyncio_wait_for(scanner._task, timeout=0.1)
            except (asyncio.TimeoutError, asyncio.CancelledError):
                pass  # Task cancellation or timeout during cleanup is acceptable
        # Attempt to stop the scanner gracefully, which should now see task as done or event set
        await scanner.stop()
        scanner._task = None  # Explicitly ensure task is cleared


@pytest.mark.asyncio
async def test_periodic_scanner_stop_when_not_running(
    periodic_scanner_fixture: PeriodicScanner,
):
    scanner = periodic_scanner_fixture
    scanner._task = None
    await scanner.stop()  # Should log and do nothing problematic


@pytest.mark.asyncio
async def test_periodic_scanner_handles_scan_exception(
    periodic_scanner_fixture: PeriodicScanner, mock_scan_use_case: MagicMock, caplog
):
    scanner = periodic_scanner_fixture
    expected_successful_scan_response = ScanResponseDTO(
        success=True, message="Scan OK", tracks_added=1, tracks_updated=0, errors=0
    )
    mock_scan_use_case.scan_directory.side_effect = [
        Exception("Scan failed!"),
        expected_successful_scan_response,
    ]

    async def custom_wait_for_side_effect_exception(awaitable, timeout):
        if not hasattr(custom_wait_for_side_effect_exception, "calls_in_loop"):
            custom_wait_for_side_effect_exception.calls_in_loop = 0

        if timeout == scanner._scan_interval:
            custom_wait_for_side_effect_exception.calls_in_loop += 1
            if (
                custom_wait_for_side_effect_exception.calls_in_loop <= 2
            ):  # Allow two timeouts for two scan attempts
                if (
                    custom_wait_for_side_effect_exception.calls_in_loop == 2
                ):  # On the 2nd scan's wait
                    scanner._stop_event.set()  # Ensure loop terminates after this 2nd scan
                raise asyncio.TimeoutError()
            # This path (calls_in_loop > 2 for loop wait) should not be hit.
            return await original_asyncio_wait_for(awaitable, timeout=0.001)
        else:
            return await original_asyncio_wait_for(awaitable, timeout=timeout)

    with patch(
        "src.mus.infrastructure.tasks.background_scanner.broadcast_sse_event",
        new_callable=AsyncMock,
    ) as mock_broadcast_sse_module_level_exc, patch(
        "asyncio.wait_for", side_effect=custom_wait_for_side_effect_exception
    ) as mock_wait_for_patch_obj_exc:
        _ = mock_wait_for_patch_obj_exc  # Explicitly use to satisfy linter
        scanner._broadcast_sse_event = (
            mock_broadcast_sse_module_level_exc  # Make the instance use the mock
        )

        await scanner.start()
        await asyncio.sleep(0.1)  # Allow time for cycles to occur based on mock

        await scanner.stop()

    assert mock_scan_use_case.scan_directory.call_count == 2
    assert any(
        "Error during scan directory operation: Scan failed!" in message
        for message in caplog.messages
    )
    mock_broadcast_sse_module_level_exc.assert_any_call(
        {
            "type": "scan_completed",
            "summary": expected_successful_scan_response.model_dump(),
        }
    )
