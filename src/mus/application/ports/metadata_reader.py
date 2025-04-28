from datetime import datetime
from pathlib import Path
from typing import Protocol


class IMetadataReader(Protocol):
    async def read_metadata(
        self, file_path: Path
    ) -> tuple[str, str, float, datetime] | None: ...
