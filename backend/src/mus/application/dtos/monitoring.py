from pydantic import BaseModel


class QueueStatsDTO(BaseModel):
    name: str
    jobs: int
