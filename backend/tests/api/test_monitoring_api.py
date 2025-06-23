import pytest
from unittest.mock import patch, MagicMock
from fastapi.testclient import TestClient

from src.mus.main import app

warning_filter = "ignore::sqlalchemy.exc.SAWarning"


@pytest.mark.filterwarnings(warning_filter)
def test_get_queue_stats():
    with (
        patch(
            "src.mus.infrastructure.api.routers.monitoring_router.get_high_priority_queue"
        ) as mock_high_queue,
        patch(
            "src.mus.infrastructure.api.routers.monitoring_router.get_low_priority_queue"
        ) as mock_low_queue,
    ):
        mock_high_queue_instance = MagicMock()
        mock_low_queue_instance = MagicMock()

        mock_high_queue_instance.__len__ = MagicMock(return_value=5)
        mock_low_queue_instance.__len__ = MagicMock(return_value=10)

        mock_high_queue.return_value = mock_high_queue_instance
        mock_low_queue.return_value = mock_low_queue_instance

        client = TestClient(app)
        response = client.get("/api/v1/monitoring/queues")

        assert response.status_code == 200
        data = response.json()
        assert len(data) == 2

        high_priority_stats = next(q for q in data if q["name"] == "high_priority")
        low_priority_stats = next(q for q in data if q["name"] == "low_priority")

        assert high_priority_stats["jobs"] == 5
        assert low_priority_stats["jobs"] == 10
