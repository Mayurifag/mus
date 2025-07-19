from src.mus.util.redis_utils import get_low_priority_queue, get_high_priority_queue


def enqueue_slow_metadata(track_id: int):
    get_low_priority_queue().enqueue(
        "src.mus.service.worker_tasks.process_slow_metadata", track_id
    )


def enqueue_file_created_from_watchdog(file_path: str):
    get_high_priority_queue().enqueue(
        "src.mus.service.worker_tasks.process_file_created", file_path
    )


def enqueue_file_created_from_upload(file_path: str):
    get_high_priority_queue().enqueue(
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
