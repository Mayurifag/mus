### **Feature Specification: Interactive Track Downloader from URL**

#### **1. User Problem & Goal**

Users want a seamless way to add new music to their library from various online sources (like YouTube, SoundCloud, etc.) by simply providing a URL. They need control over the final metadata, such as the track title and artist, before the file is permanently added to their collection. The process should be interactive, providing clear feedback on its status and allowing for user review.

#### **2. High-Level Solution Overview**

We will implement a new feature that allows users to initiate a download from a URL via a dedicated UI in the application's sidebar. This action will trigger a non-blocking backend process that uses `yt-dlp` to download the audio file into a temporary, isolated directory.

Once the download is complete, the backend will notify the frontend, which will then present the user with a confirmation modal. This modal will be pre-filled with metadata (title, artist) and cover art extracted from the downloaded file. Use modal we already have. The user can review, edit, and confirm these details.

Upon confirmation, a final backend process will apply the user-approved metadata, rename the file accordingly, and move it into the main music library. At this point, the application's existing file-watching system will detect the new track and integrate it into the library, completing the workflow.

#### **3. Detailed End-to-End Workflow**

1.  **Initiation (Frontend)**:
    *   The user accesses a new "Download from URL" section in the right sidebar.
    *   They paste a valid URL into an input field and click a "Download" button.
    *   The UI immediately shows a loading or progress indicator in the sidebar, confirming the process has started.

2.  **Download Request (Backend - API 1)**:
    *   The frontend sends a `POST` request to a new, authenticated endpoint: `/api/v1/downloads/url` with the URL in the request body.
    *   The backend validates that the input is a well-formed URL.
    *   It checks for a rate-limiting lock in Redis (e.g., `download_lock:user_session`) to ensure only one download is active at a time per user. If a lock exists, it returns a `429 Too Busy` error.
    *   If clear, it sets the Redis lock with a timeout (e.g., 10 minutes) and enqueues a `streaq` background job (`download_track_from_url`).
    *   The API immediately responds with a `202 Accepted` status, unblocking the frontend.

3.  **Background Processing (Backend - Streaq Job)**:
    *   The `streaq` worker picks up the job.
    *   It sends a `download_started` SSE event to the frontend, which can be used to update the UI status.
    *   It creates a unique temporary directory (e.g., `/app_data/tmp/downloads/<uuid>/`).
    *   It constructs and executes the `yt-dlp` command using `asyncio.create_subprocess_exec` for security. The command is configured to:
        *   Download the best audio-only format (`-f 'ba'`).
        *   Convert it to MP3 (`-x --audio-format mp3`).
        *   Embed metadata and a thumbnail.
        *   Save the output to the unique temporary directory.
    *   If `yt-dlp` fails (e.g., invalid URL, network issue), the job sends a `download_failed` SSE event with an error message, cleans up the temporary directory, and releases the Redis lock.

4.  **Ready for Review (Backend -> Frontend)**:
    *   Upon successful download, the job uses `mutagen` to read the basic title and artist tags from the newly created MP3 file.
    *   It reads the embedded thumbnail image file, converts it to a base64 Data URL, and cleans it up.
    *   It sends a `download_ready_for_review` SSE event to the frontend. This event payload includes:
        *   A temporary ID (the `<uuid>` of the folder).
        *   The suggested `title` and `artist`.
        *   The `coverDataUrl` (base64 string).

5.  **User Confirmation (Frontend)**:
    *   The frontend's event handler receives the `download_ready_for_review` event.
    *   It updates its state, removing the progress indicator.
    *   It automatically opens the `TrackMetadataModal` component, pre-filled with the title, artist, and cover art from the SSE event.

6.  **Finalization Request (Backend - API 2)**:
    *   The user reviews and edits the metadata in the modal and clicks "Save".
    *   The frontend sends a `POST` request to a second new endpoint, `/api/v1/downloads/confirm`.
    *   The request body contains the temporary ID and the final, user-approved `title` and `artist`.

7.  **Completion (Backend)**:
    *   The `/downloads/confirm` endpoint receives the request.
    *   It locates the audio file in the corresponding temporary directory.
    *   It uses `mutagen` to write the final, confirmed title and artist tags into the MP3 file.
    *   It generates the final, sanitized filename based on the confirmed metadata.
    *   It moves the finalized MP3 file from the temporary directory to the main music library directory.
    *   It performs a final cleanup of the temporary directory, deleting any remaining files.
    *   It releases the Redis lock.

8.  **Library Integration (File Watcher)**:
    *   The existing file watcher service detects a new file (`File Created`) in the music library.
    *   It triggers its standard processing pipeline: fast metadata extraction, database insertion, and enqueuing a job for slower processing (like cover resizing and tag standardization). This seamlessly integrates the new track into the user's library.

#### **4. Technical & Architectural Decisions**

*   **Asynchronous Flow**: The entire download process is asynchronous to prevent blocking the API and provide a responsive user experience for a potentially long-running operation.
*   **Temporary Directory**: Using a temporary folder is crucial. It isolates incomplete downloads, prevents the file watcher from picking up partial files, and allows for user confirmation before a track is officially added to the library. A cleanup routine will be implemented to remove stale temporary folders.
*   **Security**:
    *   **Command Injection**: `asyncio.create_subprocess_exec` will be used with a list of arguments to pass the user-provided URL safely, preventing any possibility of shell injection.
    *   **Authentication**: All new API endpoints will be protected by the application's existing authentication layer.
    *   **Rate Limiting**: A simple Redis lock will prevent a single user from overwhelming the server with multiple concurrent download requests.
*   **Configuration**: To support downloads from sites requiring login, users can mount a volume to `/app_data/yt-dlp-config/`. The backend will be configured to automatically use a `cookies.txt` file from this directory if it exists, providing a secure way for users to manage their own credentials.
*   **State Management**: A new `downloadStore` on the frontend will manage the state of the download process (`idle`, `downloading`, `awaiting_confirmation`, `failed`), driven by events from the SSE stream.
*   **Component Reuse**: The existing `TrackMetadataModal` will be extended with a new `confirm` mode to handle the review step, promoting code reuse.
