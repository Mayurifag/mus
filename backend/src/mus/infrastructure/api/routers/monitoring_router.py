from typing import List
from fastapi import APIRouter

from src.mus.application.dtos.monitoring import QueueStatsDTO
from src.mus.util.queue_utils import get_high_priority_queue, get_low_priority_queue


router = APIRouter(prefix="/api/v1/monitoring", tags=["monitoring"])


@router.get("/queues", response_model=List[QueueStatsDTO])
async def get_queue_stats() -> List[QueueStatsDTO]:
    high_priority_queue = get_high_priority_queue()
    low_priority_queue = get_low_priority_queue()

    return [
        QueueStatsDTO(name="high_priority", jobs=len(high_priority_queue)),
        QueueStatsDTO(name="low_priority", jobs=len(low_priority_queue)),
    ]
