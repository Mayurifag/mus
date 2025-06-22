from rq import Queue
from redis import Redis

from src.mus.config import settings

_low_priority_queue = None
_high_priority_queue = None


def get_low_priority_queue() -> Queue:
    global _low_priority_queue
    if _low_priority_queue is None:
        _low_priority_queue = Queue(
            "low_priority", connection=Redis.from_url(settings.DRAGONFLY_URL)
        )
    return _low_priority_queue


def get_high_priority_queue() -> Queue:
    global _high_priority_queue
    if _high_priority_queue is None:
        _high_priority_queue = Queue(
            "high_priority", connection=Redis.from_url(settings.DRAGONFLY_URL)
        )
    return _high_priority_queue


def enqueue_slow_metadata(track_id: int):
    get_low_priority_queue().enqueue(
        "src.mus.service.worker_tasks.process_slow_metadata", track_id
    )


def enqueue_file_created(file_path: str):
    get_low_priority_queue().enqueue(
        "src.mus.service.worker_tasks.process_file_created", file_path
    )


def enqueue_file_modified(file_path: str):
    get_low_priority_queue().enqueue(
        "src.mus.service.worker_tasks.process_file_modified", file_path
    )


def enqueue_file_deletion(file_path: str):
    get_high_priority_queue().enqueue(
        "src.mus.service.worker_tasks.process_file_deletion", file_path
    )


def enqueue_file_move(src_path: str, dest_path: str):
    get_high_priority_queue().enqueue(
        "src.mus.service.worker_tasks.process_file_move", src_path, dest_path
    )
