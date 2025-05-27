import pytest
from unittest.mock import AsyncMock, patch, MagicMock
import asyncio
import json

from fastapi import Request
from fastapi.responses import StreamingResponse

from src.mus.infrastructure.api.sse_handler import (
    broadcast_sse_event,
    track_updates_sse,
    active_sse_clients,
)


@pytest.mark.asyncio
async def test_broadcast_sse_event():
    # Setup mock clients
    mock_queue1 = AsyncMock(spec=asyncio.Queue)
    mock_queue2 = AsyncMock(spec=asyncio.Queue)

    # Save original clients list
    original_clients = active_sse_clients.copy()

    try:
        # Replace with our test queues
        active_sse_clients.clear()
        active_sse_clients.extend([mock_queue1, mock_queue2])

        # Test with all parameters
        await broadcast_sse_event(
            message_to_show="Test message",
            message_level="success",
            action_key="test_action",
            action_payload={"test": "data"},
        )

        # Wait for tasks to complete
        await asyncio.sleep(0.01)

        # Verify both queues received the event
        expected_data = {
            "message_to_show": "Test message",
            "message_level": "success",
            "action_key": "test_action",
            "action_payload": {"test": "data"},
        }

        mock_queue1.put.assert_awaited_once_with(expected_data)
        mock_queue2.put.assert_awaited_once_with(expected_data)

        # Test with minimal parameters
        mock_queue1.reset_mock()
        mock_queue2.reset_mock()

        await broadcast_sse_event(message_to_show="Minimal message")

        # Wait for tasks to complete
        await asyncio.sleep(0.01)

        expected_minimal_data = {
            "message_to_show": "Minimal message",
            "message_level": None,
            "action_key": None,
            "action_payload": None,
        }

        mock_queue1.put.assert_awaited_once_with(expected_minimal_data)
        mock_queue2.put.assert_awaited_once_with(expected_minimal_data)

    finally:
        # Restore original clients
        active_sse_clients.clear()
        active_sse_clients.extend(original_clients)


@pytest.mark.asyncio
async def test_track_updates_sse():
    # Mock request
    mock_request = MagicMock(spec=Request)
    mock_request.is_disconnected = AsyncMock(return_value=False)

    # Save original clients list
    original_clients = active_sse_clients.copy()
    active_sse_clients.clear()

    try:
        # Call the SSE endpoint
        response = await track_updates_sse(mock_request)

        # Verify response type
        assert isinstance(response, StreamingResponse)
        assert response.media_type == "text/event-stream"

        # Verify a client was added
        assert len(active_sse_clients) == 1

        # Test event generation
        event_generator = response.body_iterator

        # Mock queue.get to return test data then raise TimeoutError
        queue = active_sse_clients[0]

        # Add test event to the queue
        await queue.put(
            {
                "message_to_show": "Test SSE message",
                "message_level": "info",
                "action_key": "test",
                "action_payload": None,
            }
        )

        # Get the first event (our test data)
        event_data = await anext(event_generator)
        expected_event = f"data: {json.dumps({
            'message_to_show': 'Test SSE message',
            'message_level': 'info',
            'action_key': 'test',
            'action_payload': None
        })}\n\n"
        assert event_data == expected_event

        # Simulate client disconnection
        mock_request.is_disconnected = AsyncMock(return_value=True)

        # The generator should exit on next iteration
        # We need to patch asyncio.wait_for to avoid actual waiting
        with patch("asyncio.wait_for", new=AsyncMock(side_effect=asyncio.TimeoutError)):
            # This should be the last iteration due to is_disconnected=True
            with pytest.raises(StopAsyncIteration):
                await anext(event_generator)

        # Verify client was removed
        assert len(active_sse_clients) == 0

    finally:
        # Restore original clients
        active_sse_clients.clear()
        active_sse_clients.extend(original_clients)
