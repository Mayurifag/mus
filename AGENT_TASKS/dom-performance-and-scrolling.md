# Task: Granular DOM Updates & Scrolling Removal

## User Problem
The current track list re-renders entirely on any change (add, edit, delete), causing performance issues and a poor user experience. Additionally, automatic scrolling behavior interferes with manual navigation of the track list.

## High-Level Solution
Refactor the application to use event-driven, granular DOM updates for the track list. The backend will emit specific Server-Sent Events (SSE) for track additions, updates, and deletions. The frontend will handle these events by atomically updating the `trackStore`, allowing Svelte's keyed `#each` block to perform minimal DOM manipulations. All automatic scrolling functionality will be removed to give the user full control over the view.

## Success Metrics
- When a track is added, deleted, or edited, only the corresponding `TrackItem` element in the DOM is added, removed, or updated. The rest of the list remains untouched.
- The `reload_tracks` SSE event is completely removed and replaced with `track_added`, `track_updated`, and `track_deleted` events.
- The track list no longer automatically scrolls to the current track under any circumstance (on load, on track change, etc.).
- The application remains stable and functional, with all track operations correctly reflected in the UI.
- `make ci` passes successfully.

## Detailed Description
This task involves a coordinated refactoring of both backend and frontend. The backend must stop sending a generic `reload_tracks` event and instead emit specific events (`track_added`, `track_updated`, `track_deleted`) with relevant data payloads. The frontend must then be updated to handle these specific events by making atomic changes to its state stores, leveraging Svelte's reactivity for efficient DOM updates. All code related to automatic scrolling must be identified and removed from the frontend components. A new test file for `eventHandlerService.ts` will be created to validate the new event handling logic.

## Subtasks

### [x] 1. Implement Granular DOM Updates and Remove Automatic Scrolling
**Description**: Refactor backend SSE to be event-specific and update frontend to handle these events for atomic DOM changes, while simultaneously removing all auto-scroll functionality.
**Details**:

**Backend Changes:**

1.  **`src/mus/infrastructure/api/sse_handler.py`**:
    *   Import `BaseModel` from `pydantic`.
    *   Define a Pydantic model `class MusEvent(BaseModel)` to structure SSE data with fields `message_to_show`, `message_level`, `action_key`, and `action_payload`.
    *   Modify the `trigger_sse_event` endpoint to be a `POST` request. Update its signature to `async def trigger_sse_event(event_data: MusEvent):`.
    *   In `trigger_sse_event`, call `broadcast_sse_event` using the fields from the `event_data` body.
    *   In `notify_sse_from_worker`, add a `payload: Optional[Dict[str, Any]] = None` argument.
    *   Update `notify_sse_from_worker` to construct a dictionary `body` containing all event data (`action_key`, `message_to_show`, `message_level`, `action_payload`) and send it as `json=body` in the `httpx.AsyncClient.post` call. Remove the `params` dictionary.

2.  **`src/mus/util/db_utils.py`**:
    *   In `update_track_path`, modify the function to return the `track` object after committing and refreshing the session, or `None` if not found. Change the return type hint to `Optional[Track]`.

3.  **`src/mus/service/worker_tasks.py`**:
    *   In `process_slow_metadata`, update the `notify_sse_from_worker` call:
        *   Set `action_key="track_updated"`.
        *   Pass `payload=track.model_dump()`.
    *   In `process_file_deletion`, update the `notify_sse_from_worker` call:
        *   Set `action_key="track_deleted"`.
        *   Pass `payload={"id": track_id}`.
    *   In `process_file_move`, check if `updated_track` is not `None` and then call `notify_sse_from_worker`:
        *   Set `action_key="track_updated"`.
        *   Pass `payload=updated_track.model_dump()`.
    *   In `process_file_upsert`, at the end, replace the `notify_sse_from_worker` call with this logic:
        *   `action_key = "track_added" if is_creation else "track_updated"`
        *   `action_message = "Added" if is_creation else "Updated"`
        *   `level = "success" if is_creation else "info"`
        *   Call `notify_sse_from_worker` with these variables and `payload=upserted_track.model_dump()`.

4.  **`src/mus/application/use_cases/edit_track_use_case.py`**:
    *   Import `notify_sse_from_worker`.
    *   After the `add_track_history` call, add a new call: `await notify_sse_from_worker(action_key="track_updated", message=f"Updated track '{track.title}'", level="info", payload=track.model_dump())`.

**Frontend Changes:**

1.  **`src/lib/types/index.ts`**:
    *   Add a new interface: `export interface MusEvent { message_to_show: string | null; message_level: "success" | "error" | "info" | "warning" | null; action_key: string | null; action_payload: Record<string, unknown> | null; }`.

2.  **`src/lib/stores/trackStore.ts`**:
    *   Add a new method `addTrack(track: Track)`: It should prepend the new track to the `tracks` array and increment `currentTrackIndex` if it's not null.
    *   Add a new method `updateTrack(updatedTrack: Track)`: It should find and replace the track in the `tracks` array by `id`. It must also update `currentTrack` if the IDs match, and update the track within the `playHistory` array.
    *   Add a new method `deleteTrack(trackId: number)`: It should filter the track out of the `tracks` array. It must handle all edge cases: decrementing `currentTrackIndex` if a preceding track is deleted, re-calculating `currentTrack` if the deleted track was the current one (e.g., move to the next or previous), and removing the track from `playHistory`.

3.  **`frontend/src/lib/services/eventHandlerService.ts`**:
    *   Import the new `MusEvent` type.
    *   Create a helper function `createUrlsForTrack(trackData: any): Track` inside the file. This function will receive a track payload, add the full `VITE_PUBLIC_API_HOST` prefix to `cover_small_url` and `cover_original_url` if they exist, and return a fully-formed `Track` object.
    *   In `handleMusEvent(payload: MusEvent)`, replace the logic for `reload_tracks` with a `switch` statement on `payload.action_key`.
    *   Create cases for `"track_added"`, `"track_updated"`, and `"track_deleted"`.
    *   In the `'track_added'` and `'track_updated'` cases, call `createUrlsForTrack` with the `payload.action_payload` and then call the corresponding new methods on `trackStore` (`addTrack`, `updateTrack`).
    *   In the `'track_deleted'` case, call `trackStore.deleteTrack` with the `id` from the payload.

4.  **`frontend/src/lib/components/domain/TrackList.svelte`**:
    *   Remove the `onMount` and `$effect` lifecycle functions entirely.
    *   Delete the `isCurrentTrackVisible` state variable and the `observer` instance.
    *   Delete the `scrollToCurrentTrack` function.
    *   This component should now only be responsible for iterating over the `tracks` prop using a keyed `#each` block.

5.  **`frontend/src/lib/components/layout/PlayerFooter.svelte`**:
    *   Delete the `triggerManualScroll` function.
    *   Remove the `onclick` and `onkeydown` event handlers from the album cover `div` and the track title `span`.

**Testing:**
1.  **Create `frontend/src/lib/services/eventHandlerService.test.ts`**:
    *   Write unit tests for `handleMusEvent`.
    *   Mock the `trackStore` and `apiClient`.
    *   For each event (`track_added`, `track_updated`, `track_deleted`), create a mock `MusEvent` payload.
    *   Call `handleMusEvent` with the mock payload and assert that the correct `trackStore` method was called with the correctly processed data (e.g., `trackStore.addTrack` was called with a full `Track` object).
    *   Verify that toast notifications are called when `message_to_show` is present.
**Filepaths to Modify**: `src/mus/infrastructure/api/sse_handler.py,src/mus/service/worker_tasks.py,src/mus/application/use_cases/edit_track_use_case.py,src/mus/util/db_utils.py,frontend/src/lib/types/index.ts,frontend/src/lib/stores/trackStore.ts,frontend/src/lib/services/eventHandlerService.ts,frontend/src/lib/components/domain/TrackList.svelte,frontend/src/lib/components/layout/PlayerFooter.svelte,frontend/src/lib/services/eventHandlerService.test.ts`
**Relevant Make Commands (Optional)**: `make back-test, make front-test, make ci`
