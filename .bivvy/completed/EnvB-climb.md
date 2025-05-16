<Climb>
  <header>
    <id>EnvB</id>
    <type>feature</type>
    <description>Integrate .env file for backend MUSIC_DIR configuration and update .gitignore.</description>
  </header>
  <newDependencies>None. The `python-dotenv` library is expected to be available as a sub-dependency of `uvicorn[standard]`.</newDependencies>
  <prerequisitChanges>
    - The backend's `FileSystemScanner` (from task E4hN) already uses `os.getenv("MUSIC_DIR", "./music")` to determine the music directory.
    - This task focuses on enabling `python-dotenv` to populate `os.environ` from a `backend/.env` file.
  </prerequisitChanges>
  <relevantFiles>
    - `backend/.env` (New file to be created)
    - `backend/src/mus/main.py` (To be modified for `load_dotenv`)
    - `backend/src/mus/infrastructure/scanner/file_system_scanner.py` (Verification of existing behavior)
    - `.gitignore` (Root project gitignore, to be modified)
    - `backend/tests/test_dotenv_integration.py` (New test file)
    - `backend/pyproject.toml` (To confirm `python-dotenv` or its provider `uvicorn[standard]` is listed)
  </relevantFiles>
  <everythingElse>
    ## Feature Overview

    *   **Purpose Statement**: To enable configuration of the backend's music directory (`MUSIC_DIR`) through a `.env` file located in the `backend/` directory. This improves developer experience by providing a conventional way to manage local environment-specific settings and enhances deployment flexibility.
    *   **Problem Being Solved**:
        *   Currently, `MUSIC_DIR` relies on system environment variables or a hardcoded default. Using a `.env` file is a common practice for local development, making it easier to set custom paths without altering system-wide environment settings.
        *   Ensures that sensitive or environment-specific paths are not hardcoded or accidentally committed to version control.
    *   **Success Metrics**:
        *   A `backend/.env` file is created and can define `MUSIC_DIR`.
        *   The backend application (specifically `FileSystemScanner`) correctly reads and uses the `MUSIC_DIR` value from `backend/.env` if present.
        *   If `MUSIC_DIR` is not defined in `backend/.env` or system environment variables, the backend defaults to `./music`.
        *   The `backend/.env` file is successfully ignored by Git due to an updated `.gitignore` entry.
        *   All related tests, including new tests for `.env` integration, pass successfully.
        *   `make ci` passes.

    ## Requirements

    #### Functional Requirements

    1.  **`.env` File Creation**: A file named `.env` shall be created in the `backend/` directory.
    2.  **`MUSIC_DIR` Configuration**: The `backend/.env` file shall be capable of defining the `MUSIC_DIR` variable (e.g., `MUSIC_DIR=\"/custom/path/to/music\"`).
    3.  **`.env` Loading**: The backend application, specifically in `backend/src/mus/main.py`, shall be modified to load environment variables from `backend/.env` at startup using the `load_dotenv()` function from the `python-dotenv` library. This loading must occur before the `MUSIC_DIR` variable is accessed by other parts of the application. The path to the `.env` file must be specified robustly (e.g., `Path(__file__).resolve().parent.parent.parent / \".env\"`).
    4.  **`FileSystemScanner` Usage**: The `backend/src/mus/infrastructure/scanner/file_system_scanner.py` must continue to use `os.getenv("MUSIC_DIR", "./music")` to obtain the music directory path. The `os.makedirs` call for this directory must remain functional.
    5.  **`.gitignore` Update**: The root `.gitignore` file shall be updated to include `/backend/.env` to ensure this file is not tracked by Git.

    #### Technical Requirements

    *   **Library**: The `python-dotenv` library shall be used for loading the `.env` file.
    *   **Loading Point**: `load_dotenv()` must be called early in the application's lifecycle, typically at the beginning of `backend/src/mus/main.py`.
    *   **Variable Name**: The environment variable name must be `MUSIC_DIR` for consistency with existing code.

    ## Development Details

    *   **`.env` File Path**: The `.env` file will reside at `backend/.env`.
    *   **Loading Logic in `main.py`**:
        ```python
        # In backend/src/mus/main.py
        from pathlib import Path
        from dotenv import load_dotenv

        # Path to backend/.env from backend/src/mus/main.py
        dotenv_path = Path(__file__).resolve().parent.parent.parent / ".env"
        load_dotenv(dotenv_path=dotenv_path)
        # ... rest of application setup