import os
import pytest
from pathlib import Path
from dotenv import load_dotenv
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner


@pytest.fixture
def temp_env_file(tmp_path):
    """Create a temporary .env file with custom MUSIC_DIR path."""
    env_path = tmp_path / ".env"
    test_music_path = tmp_path / "temp_music_for_dotenv_test"
    os.makedirs(test_music_path, exist_ok=True)

    with open(env_path, "w") as f:
        f.write(f'MUSIC_DIR="{test_music_path}"')

    return {"env_path": env_path, "music_dir": str(test_music_path)}


@pytest.fixture
def env_without_music_dir(tmp_path):
    """Create a temporary .env file without MUSIC_DIR."""
    env_path = tmp_path / ".env"

    with open(env_path, "w") as f:
        f.write("# Empty .env file without MUSIC_DIR")

    return {"env_path": env_path}


def test_scanner_uses_music_dir_from_dotenv(temp_env_file, monkeypatch):
    """Test that FileSystemScanner uses MUSIC_DIR from .env file."""
    # First unset any existing MUSIC_DIR from the environment
    monkeypatch.delenv("MUSIC_DIR", raising=False)

    # Set the environment variable directly to ensure it's set properly
    monkeypatch.setenv("MUSIC_DIR", str(temp_env_file["music_dir"]))

    # Create a FileSystemScanner instance which should use the MUSIC_DIR from env
    scanner = FileSystemScanner()

    # Verify that the scanner uses the custom path
    assert scanner.MUSIC_DIR == temp_env_file["music_dir"]
    assert scanner.root_dir == Path(temp_env_file["music_dir"])

    # Verify that the directory was created
    assert os.path.exists(temp_env_file["music_dir"])


def test_scanner_uses_default_when_no_env_music_dir(env_without_music_dir, monkeypatch):
    """Test that FileSystemScanner defaults to ./music when MUSIC_DIR is not set."""
    # Ensure MUSIC_DIR is not set in the environment
    monkeypatch.delenv("MUSIC_DIR", raising=False)

    # Load the .env file without MUSIC_DIR
    load_dotenv(dotenv_path=env_without_music_dir["env_path"])

    # Create a FileSystemScanner instance
    scanner = FileSystemScanner()

    # Verify that the scanner uses the default path
    assert scanner.MUSIC_DIR == "./music"
    assert scanner.root_dir == Path("./music")

    # Clean up the default directory if it was created
    if os.path.exists("./music"):
        os.rmdir("./music")


def test_scanner_no_env_file(monkeypatch):
    """Test that FileSystemScanner works when no .env file exists."""
    # Ensure MUSIC_DIR is not set in the environment
    monkeypatch.delenv("MUSIC_DIR", raising=False)

    # Create a non-existent path for the .env file
    non_existent_env = Path("/non_existent_path/.env")

    # Try to load the non-existent .env file (should not raise an error)
    load_dotenv(dotenv_path=non_existent_env)

    # Create a FileSystemScanner instance
    scanner = FileSystemScanner()

    # Verify that the scanner uses the default path
    assert scanner.MUSIC_DIR == "./music"
    assert scanner.root_dir == Path("./music")

    # Clean up the default directory if it was created
    if os.path.exists("./music"):
        os.rmdir("./music")
