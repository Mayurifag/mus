import asyncio
import json
import logging
import os
from typing import List, Dict, Any, Optional, Literal

from fastapi import APIRouter, Request
from fastapi.responses import StreamingResponse
from pydantic import BaseModel
import httpx

router = APIRouter(prefix="/api/v1/events", tags=["events"])

active_sse_clients: List[asyncio.Queue] = []

MessageLevel = Literal["success", "error", "info", "warning"]


class MusEvent(BaseModel):
    message_to_show: Optional[str] = None
    message_level: Optional[MessageLevel] = None
    action_key: Optional[str] = None
    action_payload: Optional[Dict[str, Any]] = None


async def broadcast_sse_event(
    message_to_show: Optional[str] = None,
    message_level: Optional[MessageLevel] = None,
    action_key: Optional[str] = None,
    action_payload: Optional[Dict[str, Any]] = None,
):
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
                if await request.is_disconnected():
                    break

                try:
                    event_data = await asyncio.wait_for(client_queue.get(), timeout=1.0)
                    yield f"data: {json.dumps(event_data)}\n\n"
                except asyncio.TimeoutError:
                    yield ": keepalive\n\n"

        except asyncio.CancelledError:
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


@router.post("/trigger")
async def trigger_sse_event(event_data: MusEvent):
    await broadcast_sse_event(
        message_to_show=event_data.message_to_show,
        message_level=event_data.message_level,
        action_key=event_data.action_key,
        action_payload=event_data.action_payload,
    )
    return {"status": "ok"}


async def notify_sse_from_worker(
    action_key: str,
    message: Optional[str] = None,
    level: Optional[MessageLevel] = None,
    payload: Optional[Dict[str, Any]] = None,
):
    backend_url = os.getenv("BACKEND_URL", "http://127.0.0.1:8001")
    try:
        async with httpx.AsyncClient() as client:
            body = {
                "action_key": action_key,
                "message_to_show": message,
                "message_level": level,
                "action_payload": payload,
            }
            await client.post(
                f"{backend_url}/api/v1/events/trigger", json=body, timeout=1.0
            )
    except Exception as e:
        logging.getLogger(__name__).debug(f"Failed to send SSE notification: {e}")
