# Implement Frontend for YT-DLP Downloader

## User Problem
Users cannot add new music to their library from online sources like YouTube by providing a URL. The process should be interactive, providing clear feedback on its status and allowing for user review and metadata editing before the track is added.

## High-Level Solution
This task finalizes the `yt-dlp` download feature by implementing the required frontend components and logic. A new "Download from URL" UI will be added to the `RightSidebar`. This component will accept a URL, trigger a backend download process, and listen for Server-Sent Events (SSE) to provide real-time feedback. A new store will manage the download state. When a download is complete, the existing `TrackMetadataModal` will be used to allow the user to review and edit the track's metadata before it is permanently added to the library.

## Success Metrics
- A "Download from URL" UI is present and functional in the `RightSidebar`.
- Users can input a URL and initiate a download, which calls the correct backend API.
- The UI provides real-time feedback on the download status (e.g., 'downloading', 'failed', 'ready for review').
- A new `downloadStore` correctly manages the state of the download process.
- Upon successful download, the `TrackMetadataModal` opens, pre-filled with extracted metadata and cover art.
- Confirming the metadata in the modal finalizes the download and adds the track to the library.
- The entire process is non-blocking and provides clear user feedback via toasts and UI state changes.

## Detailed Description
This implementation builds upon the backend infrastructure for downloading tracks from URLs using `yt-dlp`.

**Architecture:**
- **`DownloadManager.svelte`**: A new component responsible for the download UI (input field, button) and displaying the download status. It will be placed in the `RightSidebar`.
- **`downloadStore.ts`**: A new Svelte store to manage the global state of the download process. It will track the status (`idle`, `downloading`, `awaiting_review`, `failed`), any error messages, and the data required for the confirmation modal (temporary ID, suggested metadata, cover art).
- **`apiClient.ts`**: Will be extended with two new functions: one to initiate the download (`/api/v1/downloads/url`) and another to confirm it (`/api/v1/downloads/confirm`).
- **`eventHandlerService.ts`**: The existing SSE handler will be updated to process new download-related events (`download_started`, `download_failed`, `download_ready_for_review`) and update the `downloadStore`.
- **`TrackMetadataModal.svelte`**: The existing modal will be adapted to handle the "confirm download" use case.

**API & SSE Contract:**
- **`POST /api/v1/downloads/url`**: Initiates a download.
  - Body: `{ "url": "string" }`
  - Success Response: `202 Accepted`
- **`POST /api/v1/downloads/confirm`**: Finalizes a download.
  - Body: `{ "tempId": "string", "title": "string", "artist": "string" }`
  - Success Response: `200 OK`
- **SSE Events**:
  - `download_started`: Payload is empty. UI should show a loading state.
  - `download_failed`: Payload: `{ "error": "string" }`. UI should show an error.
  - `download_ready_for_review`: Payload: `{ "tempId": "string", "title": "string", "artist": "string", "coverDataUrl": "string" }`. UI should open the confirmation modal.

## Subtasks

### [x] 1. Implement Frontend UI, State Management, and Finalization Logic
**Description**: Create the full frontend experience for the downloader, including the UI in the sidebar, state management via a new store, handling SSE events, and integrating with the metadata modal for final confirmation.
**Details**:
1.  **Create Store**: Implement `src/lib/stores/downloadStore.ts` to manage the download state (`idle`, `downloading`, `awaiting_review`, `failed`), error message, and the payload from the `download_ready_for_review` event.
2.  **Create UI Component**: Create `src/lib/components/domain/DownloadManager.svelte`. This component will contain a form with an input for the URL and a "Download" button. It must disable the button and show a status message/spinner when the store's state is `downloading`. It should also display any error messages from the store.
3.  **Integrate Component**: Add the `DownloadManager` component into `src/lib/components/layout/RightSidebar.svelte`.
4.  **Update API Client**: In `src/lib/services/apiClient.ts`, add two new functions: `startDownload(url: string)` to call `POST /api/v1/downloads/url` and `confirmDownload(tempId: string, title: string, artist: string)` to call `POST /api/v1/downloads/confirm`.
5.  **Update Event Handler**: In `src/lib/services/eventHandlerService.ts`, update `handleMusEvent` to process the new `action_key` values (`download_started`, `download_failed`, `download_ready_for_review`) and update the `downloadStore` accordingly. Add the new event types to `src/lib/types/index.ts`.
6.  **Integrate Modal**: In `src/routes/(app)/+layout.svelte`, use a reactive statement (`$effect`) to watch the `downloadStore`. When the state becomes `awaiting_review`, open the `TrackMetadataModal` and pass the necessary data (suggested metadata, cover art) as props. The modal's 'Save' action should now call the `confirmDownload` API function.
7.  **Test**: Add tests for the `downloadStore` and the `DownloadManager` component to ensure correct state transitions and UI updates.
**Filepaths to Modify**: `src/lib/components/domain/DownloadManager.svelte`,`src/lib/stores/downloadStore.ts`,`src/lib/components/layout/RightSidebar.svelte`,`src/lib/services/apiClient.ts`,`src/lib/services/eventHandlerService.ts`,`src/routes/(app)/+layout.svelte`,`src/lib/types/index.ts`
**Relevant Make Commands (Optional)**: `make front-test, make front-lint, make ci`
