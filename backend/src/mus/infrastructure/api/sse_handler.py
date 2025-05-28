import asyncio
import json
from typing import List, Dict, Any, Optional, Literal

from fastapi import APIRouter, Request
from fastapi.responses import StreamingResponse

router = APIRouter(prefix="/api/v1/events", tags=["events"])

active_sse_clients: List[asyncio.Queue] = []

MessageLevel = Literal["success", "error", "info", "warning"]


async def broadcast_sse_event(
    message_to_show: Optional[str] = None,
    message_level: Optional[MessageLevel] = None,
    action_key: Optional[str] = None,
    action_payload: Optional[Dict[str, Any]] = None,
):
    """
    Broadcast a generic SSE event to all connected clients.

    Args:
        message_to_show: Optional message to display to the user
        message_level: Optional level of the message (success, error, info, warning)
        action_key: Optional key identifying the action to take (e.g., 'reload_tracks')
        action_payload: Optional data associated with the action
    """
    event_data = {
        "message_to_show": message_to_show,
        "message_level": message_level,
        "action_key": action_key,
        "action_payload": action_payload,
    }

    for q in active_sse_clients:
        asyncio.create_task(q.put(event_data))


@router.get("/track-updates")
async def track_updates_sse(request: Request):
    client_queue = asyncio.Queue()
    active_sse_clients.append(client_queue)

    async def event_generator():
        try:
            while True:
                # Check if client is still connected
                if await request.is_disconnected():
                    break

                try:
                    event_data = await asyncio.wait_for(client_queue.get(), timeout=1.0)
                    yield f"data: {json.dumps(event_data)}\n\n"
                except asyncio.TimeoutError:
                    # Send a comment to keep connection alive if no real data
                    yield ": keepalive\n\n"

        except asyncio.CancelledError:
            # Handle task cancellation (e.g. server shutdown)
            pass
        finally:
            if client_queue in active_sse_clients:
                active_sse_clients.remove(client_queue)

    return StreamingResponse(
        event_generator(),
        media_type="text/event-stream",
        headers={
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
        },
    )


@router.get("/test_toast")
async def test_toast():
    """
    Test endpoint to send various SSE toast notification events.
    Purely for testing the frontend toast notification system.
    """
    test_events = [
        {
            "message_to_show": "Track scan completed successfully!",
            "message_level": "success",
            # "action_key": "reload_tracks",
            "action_key": None,
            "action_payload": {"tracks_added": 5},
        }
    ]

    for event in test_events:
        asyncio.create_task(
            broadcast_sse_event(
                message_to_show=event["message_to_show"],
                message_level=event["message_level"],
                action_key=event["action_key"],
                action_payload=event["action_payload"],
            )
        )

    return {"message": "Test toast events sent", "events_count": len(test_events)}
