"""Progress parser domain service.

Pure function to parse yt-dlp --newline --progress output lines into
structured progress data dicts.  No I/O, no side-effects.
"""

import re

# In-progress: "[download]   X.X% of ~Y.YYMiB at  Z.ZZMiB/s ETA HH:MM"
_IN_PROGRESS_RE = re.compile(
    r"\[download\]\s+"
    r"(?P<percent>\d+(?:\.\d+)?)%\s+of\s+~?\S+\s+at\s+"
    r"(?P<speed>\S+(?:\s+\S+)?)\s+ETA\s+"
    r"(?P<eta>\S+)"
)

# Completion: "[download] 100% of   Y.YYMiB in 00:HH:MM"
_COMPLETE_RE = re.compile(r"\[download\]\s+100%\s+of\s+\S+\s+in\s+\S+")


def parse_progress_line(line: str) -> dict | None:
    """Parse a single yt-dlp output line into a progress dict.

    Returns ``{"percent": float, "speed": str, "eta": str}`` for recognised
    progress lines, or ``None`` for everything else (info lines, audio
    extraction lines, empty strings, etc.).
    """
    m = _IN_PROGRESS_RE.search(line)
    if m:
        return {
            "percent": float(m.group("percent")),
            "speed": m.group("speed").strip(),
            "eta": m.group("eta").strip(),
        }

    if _COMPLETE_RE.search(line):
        return {"percent": 100.0, "speed": "", "eta": ""}

    return None
