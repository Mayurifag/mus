from pathlib import Path
from typing import Optional, Dict, Any

from mutagen._file import File as MutagenFile


def extract_fast_metadata(file_path: Path) -> Optional[Dict[str, Any]]:
    try:
        audio = MutagenFile(file_path, easy=True)
        if not audio:
            return None

        stat_info = file_path.stat()
        title = (audio.get("title") or [file_path.stem])[0]
        artist = (audio.get("artist") or ["Unknown Artist"])[0]

        duration = 0
        if audio.info and (length := getattr(audio.info, "length", None)):
            duration = int(length) if length > 0 else 0

        return {
            "title": str(title),
            "artist": str(artist),
            "duration": duration,
            "inode": stat_info.st_ino,
            "added_at": int(stat_info.st_mtime),
        }
    except Exception:
        return None
