import asyncio
import json
from typing import List, Dict, Any

from fastapi import APIRouter, Request
from fastapi.responses import StreamingResponse

router = APIRouter(prefix="/api/v1/events", tags=["events"])

active_sse_clients: List[asyncio.Queue] = []


async def broadcast_sse_event(event_data: Dict[str, Any]):
    for q in active_sse_clients:
        await q.put(event_data)


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

    return StreamingResponse(event_generator(), media_type="text/event-stream")
