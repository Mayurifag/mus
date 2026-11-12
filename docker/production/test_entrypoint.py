"""Run inside the production image as root, without mounted application data."""

import os
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from urllib.request import urlopen


class EntrypointTests(unittest.TestCase):
    def test_storage_initialization(self):
        for uid, legacy, existing_cache in [
            (1000, True, False),
            (0, True, False),
            (1000, False, False),
            (1000, True, True),
        ]:
            with (
                self.subTest(uid=uid, legacy=legacy, existing_cache=existing_cache),
                tempfile.TemporaryDirectory() as directory,
            ):
                data = Path(directory)
                music = data / "music"
                music.mkdir()
                os.chown(music, uid, uid)
                cache = data / ".cache"
                if legacy:
                    self.create_database(data / "mus.db", "legacy")
                    (data / "covers").mkdir()
                    (data / "covers" / "fixture.webp").write_bytes(b"cover")
                if existing_cache:
                    cache.mkdir()
                    self.create_database(cache / "mus.db", "current")
                with tempfile.TemporaryFile() as log:
                    process = subprocess.Popen(
                        ["/app/entrypoint.sh", "/app/mus-backend"],
                        env={**os.environ, "DATA_DIR_PATH": directory},
                        stdout=log,
                        stderr=subprocess.STDOUT,
                    )
                    try:
                        self.wait_for_health(process, log)
                        with sqlite3.connect(cache / "mus.db") as connection:
                            if legacy:
                                value = connection.execute(
                                    "SELECT value FROM committed_data"
                                ).fetchone()[0]
                                self.assertEqual(
                                    value, "current" if existing_cache else "legacy"
                                )
                        self.assertEqual((cache / "mus.db").stat().st_uid, uid)
                        if legacy:
                            self.assertEqual(
                                (cache / "covers" / "fixture.webp").read_bytes(),
                                b"cover",
                            )
                            self.assertEqual((data / "mus.db").exists(), existing_cache)
                    finally:
                        process.terminate()
                        process.wait(timeout=10)

    @staticmethod
    def create_database(path, value):
        subprocess.run(
            [
                sys.executable,
                "-c",
                (
                    "import os, sqlite3, sys; "
                    "db = sqlite3.connect(sys.argv[1]); "
                    "db.executescript('PRAGMA journal_mode=WAL; "
                    "PRAGMA wal_autocheckpoint=0; "
                    "CREATE TABLE committed_data (value TEXT)'); "
                    "db.execute('INSERT INTO committed_data VALUES (?)', (sys.argv[2],)); "
                    "db.commit(); os._exit(0)"
                ),
                str(path),
                value,
            ],
            check=True,
        )
        assert Path(f"{path}-wal").stat().st_size > 0

    def wait_for_health(self, process, log):
        deadline = time.monotonic() + 20
        while process.poll() is None and time.monotonic() < deadline:
            try:
                with urlopen("http://127.0.0.1:8000/api/healthcheck.json", timeout=1):
                    return
            except OSError:
                time.sleep(0.1)
        log.seek(0)
        self.fail(log.read().decode())


if __name__ == "__main__":
    unittest.main()
