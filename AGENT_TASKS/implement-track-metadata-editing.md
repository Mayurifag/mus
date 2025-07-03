# Implement In-App Track Metadata and Filename Editing

## User Problem
Users cannot correct or update the metadata (e.g., title, artist) of their music files directly within the application. They also cannot rename the associated audio files based on the corrected metadata. This forces them to use external tools, leading to a disconnected user experience and potential inconsistencies between the library database and the actual files.

## High-Level Solution
A unified, end-to-end feature will be built to allow metadata editing. This involves creating a new API endpoint on the backend to handle file and tag modifications, and a corresponding modal-based UI on the frontend. The backend will first check for write permissions on startup and disable the feature if the music directory is read-only. The `TrackHistory` model will be enhanced to store detailed change deltas and full snapshots for every edit. The frontend will fetch the permission status and conditionally render an "Edit" option in a new context menu for each track, which opens a comprehensive editing modal.

## Success Metrics
-   Users can successfully edit the title and artist of any track through the UI.
-   The underlying audio file's ID3 tags are correctly updated with the new information, using ID3v2.4 and UTF-8 encoding.
-   The audio file is correctly renamed on the filesystem if the user chooses to do so, following the `Artist 1, Artist 2 - Title.ext` convention.
-   A new entry is created in the `TrackHistory` table for every successful edit, capturing the changes and the full state.
-   The `PATCH` endpoint correctly handles partial updates and performs no action if no changes are submitted.
-   All editing UI elements are disabled if the backend reports that the music directory is read-only.
-   The application remains stable, and all existing tests, along with new tests for this feature, pass `make ci`.

## Detailed Description
This task covers the full-stack implementation of the metadata editing feature.

**Backend Architecture:**
-   **Permissions:** A new `PermissionsService` at `src/mus/service/permissions_service.py` will perform a one-time, cached write-permission check on the music directory during application startup (in `main.py`'s lifespan). A new router, `permissions_router.py`, will expose this status via a `GET /api/v1/system/permissions` endpoint.
-   **Models & DTOs:**
    -   `TrackListDTO` in `src/mus/application/dtos/track.py` will be updated to include `file_path`.
    -   A new `TrackUpdateDTO` will be created for the `PATCH` request body, containing optional `title`, `artist`, and a `rename_file` boolean.
    -   The `TrackHistory` model in `src/mus/domain/entities/track_history.py` will be enhanced with `changes` (JSON for delta), `full_snapshot` (JSON for complete metadata), `filename` (string), and `event_type` (string).
-   **Use Case:** A new `EditTrackUseCase` at `src/mus/application/use_cases/edit_track_use_case.py` will encapsulate all business logic. This includes fetching the track, checking file permissions, calculating the change delta, using `mutagen` to write ID3 tags (handling multiple artists with a `;` separator), renaming the file (sanitizing the name and handling multiple artists with `, `), updating the `Track` record in the database, and creating a detailed `TrackHistory` entry.
-   **API:** A new `PATCH /api/v1/tracks/{track_id}` endpoint in `src/mus/infrastructure/api/routers/track_router.py` will receive the `TrackUpdateDTO`, invoke the `EditTrackUseCase`, and handle all responses and exceptions.
-   **Initial History:** The `process_file_upsert` task in `src/mus/service/worker_tasks.py` will be modified to create an initial `TrackHistory` record with `event_type='initial_scan'` when a new track is first discovered.

**Frontend Architecture:**
-   **Permissions:** A new store, `src/lib/stores/permissionsStore.ts`, will cache the `can_write_files` status fetched via `+layout.server.ts` and initialized in `+layout.svelte`.
-   **UI Trigger:** The `TrackItem.svelte` component will feature a new three-dot context menu (using `bits-ui` Popover/Dropdown and `lucide-svelte`'s `MoreVertical` icon). This menu will contain an "Edit" button, which will be disabled if `$permissionsStore.can_write_files` is false.
-   **Editing Modal:** A new `EditTrackModal.svelte` component will provide the editing interface. It will be a `bits-ui` Dialog, showing a form with inputs for title and artist. It will also have a "Rename file" checkbox (enabled by default) and a live preview of the generated filename. The "Save" button will be disabled until changes are made. The modal will prompt for confirmation before closing if there are unsaved changes.
-   **API Integration:** The modal will be responsible for calculating the changes and sending a `PATCH` request to the backend. It will use `svelte-sonner` to provide feedback on the operation's success or failure.

## Subtasks

### [ ] 1. End-to-End Implementation of Metadata Editing
**Description**: Implement the complete metadata editing feature, from backend permissions and API logic to the frontend UI components and state management.
**Details**:

**Part 1: Backend Implementation**

1.  **Permissions Service & Endpoint:**
    -   Create `src/mus/service/permissions_service.py`.
    -   Define a class `PermissionsService`. Inside, create a `_is_writable: Optional[bool] = None` instance attribute.
    -   Implement a method `check_write_permissions(self) -> bool`. If `_is_writable` is not `None`, return the cached value. Otherwise, attempt to `Path(settings.MUSIC_DIR_PATH / ".writable_check").touch()` and then `Path(settings.MUSIC_DIR_PATH / ".writable_check").unlink()`. Set `_is_writable` to `True` on success or `False` inside a `try...except OSError`. Return the result.
    -   In `src/mus/main.py`'s `lifespan` manager, create an instance of `PermissionsService` and call `await asyncio.to_thread(permissions_service.check_write_permissions)`. Make this service instance available via a dependency injection pattern for the new router.
    -   Create `src/mus/infrastructure/api/routers/permissions_router.py`.
    -   Add a `GET /api/v1/system/permissions` endpoint that depends on the `PermissionsService` and returns `{"can_write_files": permissions_service.check_write_permissions()}`.
    -   In `src/mus/main.py`, include the new `permissions_router`.

2.  **Model and DTO Enhancements:**
    -   In `src/mus/application/dtos/track.py`, modify `TrackListDTO` to include `file_path: str`.
    -   In `src/mus/infrastructure/persistence/sqlite_track_repository.py`, update the `get_all` method's `select` statement to include `Track.file_path`.
    -   In `src/mus/domain/entities/track_history.py`, import `from sqlalchemy import Column, JSON`. Modify the `TrackHistory` model to include: `changes: Optional[dict] = Field(default=None, sa_column=Column(JSON))`, `full_snapshot: Optional[dict] = Field(default=None, sa_column=Column(JSON))`, `filename: str`, and `event_type: str`.
    -   In `src/mus/service/worker_tasks.py`, within the `process_file_upsert` function, locate the `if existing_track and ...` block that creates a history entry for updates. Before that, add a block for new tracks: `if not existing_track and upserted_track and upserted_track.id:`.
    -   Inside this new block, create a `TrackHistory` instance for the initial scan:
        -   `track_id=upserted_track.id`
        -   `event_type='initial_scan'`
        -   `changes=None`
        -   `filename=Path(upserted_track.file_path).name`
        -   `title`, `artist`, `duration` from the `upserted_track`.
        -   `changed_at` set to `int(time.time())`.
        -   `full_snapshot` containing a dictionary of all relevant metadata from the `upserted_track`.
        -   Call `await add_track_history(history_entry)`.

3.  **Core Editing Logic and API Endpoint:**
    -   In `src/mus/application/dtos/track.py`, define a new pydantic model `class TrackUpdateDTO(BaseModel): title: Optional[str] = None; artist: Optional[str] = None; rename_file: bool = False`.
    -   Create the file `src/mus/application/use_cases/edit_track_use_case.py`.
    -   Define an `EditTrackUseCase` class with an `async def execute(self, track_id: int, update_data: TrackUpdateDTO)` method. This method will require repositories injected in `__init__`.
    -   **`execute` method logic:**
        1.  Fetch the track: `track = await self.track_repo.get_by_id(track_id)`. Raise `HTTPException(404)` if `not track`.
        2.  Check file writability: `if not os.access(track.file_path, os.W_OK): raise HTTPException(403)`.
        3.  Calculate changes: `changes_delta = update_data.model_dump(exclude_unset=True, exclude={'rename_file'})`. If a field in `update_data` is the same as in `track`, remove it from `changes_delta`.
        4.  Handle no-op: `if not changes_delta and not update_data.rename_file: return {"status": "no_changes"}`.
        5.  Open the audio file: `audio = mutagen.File(track.file_path, easy=True)`. Use a `try...except FileNotFoundError` block.
        6.  Update tags if changes exist: If `'title'` in `changes_delta`, set `audio['title'] = update_data.title`. If `'artist'` in `changes_delta`, set `audio['artist'] = update_data.artist`. Save with `audio.save()`.
        7.  Handle file renaming if `update_data.rename_file`:
            -   Define a sanitization function: `def sanitize(name): return re.sub(r'[<>:"/\\|?*]', '', name)`.
            -   Get new artist/title, falling back to existing `track` data if not provided in `update_data`.
            -   Format artists: `formatted_artists = ", ".join(new_artist.split(';'))`.
            -   Create new filename: `new_name = f"{sanitize(formatted_artists)} - {sanitize(new_title)}{Path(track.file_path).suffix}"`.
            -   Enforce length: `if len(new_name) > 255: raise HTTPException(400, "Filename too long")`.
            -   Perform rename: `new_path = Path(track.file_path).parent / new_name; os.rename(track.file_path, new_path)`. Update `track.file_path = str(new_path)`.
        8.  Update the `track` object in the database with the new metadata and potentially new `file_path`.
        9.  Create and save a new `TrackHistory` entry with `event_type='edit'`, the calculated `changes_delta`, the new `filename`, and a `full_snapshot` of the updated `track` object.
    -   In `src/mus/infrastructure/api/routers/track_router.py`, create the `PATCH /api/v1/tracks/{track_id}` endpoint that injects and calls the `EditTrackUseCase`.
    -   Add extensive unit and integration tests covering all paths of the `EditTrackUseCase`.

**Part 2: Frontend Implementation**

1.  **Permissions and State Management:**
    -   Create `src/lib/stores/permissionsStore.ts`. Define a writable store: `export const permissionsStore = writable({ can_write_files: false });`.
    -   In `src/routes/(app)/+layout.server.ts`, add a `fetch` call to the new `/api/v1/system/permissions` endpoint and return its result, e.g., `const permissions = await (await fetch(...)).json(); return { ..., permissions }`.
    -   In `src/routes/(app)/+layout.svelte`, inside `onMount`, call `permissionsStore.set(data.permissions)`.
    -   In `src/lib/types/index.ts`, add `file_path: string;` to the `Track` interface.
    -   In `src/lib/services/apiClient.ts`, ensure `fetchTracks` correctly maps and returns the `file_path` field for each track.

2.  **UI Component Implementation:**
    -   **Checkbox Component:** Create `src/lib/components/ui/checkbox/` directory. Add `index.ts` and `checkbox.svelte` based on `shadcn-svelte` primitives for `Checkbox`.
    -   **TrackItem Context Menu:**
        -   In `src/lib/components/domain/TrackItem.svelte`, import `DropdownMenu` from `bits-ui` and `MoreVertical` from `lucide-svelte`. Import `permissionsStore`.
        -   Add a `div` that stops event propagation (`onclick|onkeydown={e => e.stopPropagation()}`). Inside, add a `DropdownMenu.Trigger` with the `MoreVertical` icon. Bind its `disabled` attribute to `!$permissionsStore.can_write_files`.
        -   Inside the `DropdownMenu.Content`, add a `DropdownMenu.Item` labeled "Edit". On select, it should open the edit modal for the current `track`.
    -   **Edit Modal Component (`EditTrackModal.svelte`):**
        -   Create `src/lib/components/domain/EditTrackModal.svelte`.
        -   Use `Dialog` from `bits-ui`. It should accept `open` (bindable) and `track` as props.
        -   Inside, use `$state` to store the initial `track` prop and the current form values: `let initialTrack = $state(track); let formState = $state({ title: track.title, artist: track.artist, rename_file: true });`.
        -   Use `$derived` to check for changes: `let hasChanges = $derived(formState.title !== initialTrack.title || formState.artist !== initialTrack.artist || formState.rename_file !== true);`.
        -   Create a form with a text `Input` for `title` and `artist`, bound to `formState`.
        -   Add the new `Checkbox` component bound to `formState.rename_file`.
        -   Display a reactive filename preview: `let filenamePreview = $derived(...)`.
        -   The "Save" button's `disabled` prop should be `{!hasChanges}`.
        -   Implement the "onbeforeclose" handler on `Dialog.Root`: `onbeforeclose={(e) => { if (hasChanges && !window.confirm('...')) e.preventDefault(); }}`.
3.  **API Integration and Final Touches:**
    -   Create a new function `updateTrack` in `src/lib/services/apiClient.ts`. It should take `trackId` and a payload of changes. It will perform a `PATCH` request.
    -   In `EditTrackModal.svelte`, the "Save" button's `onclick` handler will calculate the delta between `formState` and `initialTrack`, call `apiClient.updateTrack`, display a success/error toast using `svelte-sonner`, and close the modal on success.
    -   Add Vitest unit tests for the new components and stores.

**Filepaths to Modify**: `src/mus/application/dtos/track.py,src/mus/infrastructure/persistence/sqlite_track_repository.py,src/mus/domain/entities/track_history.py,src/mus/service/worker_tasks.py,src/mus/main.py,src/mus/infrastructure/api/routers/track_router.py,backend/tests/api/test_track_api.py,src/lib/types/index.ts,src/lib/services/apiClient.ts,src/routes/(app)/+layout.server.ts,src/routes/(app)/+layout.svelte,src/lib/components/domain/TrackItem.svelte`
**Filepaths to Create**: `src/mus/service/permissions_service.py,src/mus/infrastructure/api/routers/permissions_router.py,src/mus/application/use_cases/edit_track_use_case.py,backend/tests/application/use_cases/test_edit_track_use_case.py,src/lib/stores/permissionsStore.ts,src/lib/components/domain/EditTrackModal.svelte,src/lib/components/ui/checkbox/index.ts,src/lib/components/ui/checkbox/checkbox.svelte`
**Relevant Make Commands (Optional)**: `make ci`
