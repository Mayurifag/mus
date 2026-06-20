# Category Agent Brief

## Goal

- Add two hardcoded categories: `Gachi` and `AI cover`.
- Detect `Right version` and `AI cover` from normalized track title/filename.
- Keep filenames consistent: `Artist - Title (Right version).mp3`.
- Keep AI-cover filenames consistent: `Artist - Title (AI cover).mp3`.
- UI should color `Right version` and `AI cover` tracks differently.
- Sidebar should show `Category` after `Artists`, with `Gachi (count)`.
- Sidebar should also show `AI cover (count)` when present.
- Hide the whole `Category` section when no category has tracks.
- Hide any individual category with count `0`.
- Remove the `Controls` div/section from the interface.

## Constraints

- Use `make` targets only.
- Do not run `cargo`, `npm`, `vitest`, or `docker compose` directly.
- Do not add app auth.
- Do not create temp/replacement files inside mounted music folders.
- For OpenCloud-mounted music, preserve existing in-place update behavior.
- Keep this minimal; do not build a generic category system unless needed.

## Naming

- Normalized right-version marker is exactly `(Right version)`.
- Normalized AI-cover marker is exactly `(AI cover)`.
- No brackets, lowercase variants, hyphen variants, or extra symbols in final filenames.
- Detection should accept common input variants:
  - `(right version)`
  - `[right version]`
  - `right ver`
  - `right-version`
  - `RIGHT VERSION`
- AI-cover detection should accept common input variants:
  - `(ai cover)`
  - `[ai cover]`
  - `ai-cover`
  - `AI COVER`
  - `a.i. cover`
- Normalization output should be only `(Right version)` and/or `(AI cover)`.
- Do not make duplicate final paths.

## Data Model

- Use normalized tags, not boolean columns.
- Add `tag` and `track_tag` tables.
- Seed/ensure two tags for now:
  - `gachi` -> `Gachi`
  - `ai-cover` -> `AI cover`
- Keep UI categories hardcoded to those two tags for now.
- Expose `tags` in backend `Track` and `TrackDto`.
- Expose `tags` in frontend `Track` type.

Schema:

~~~sql
CREATE TABLE tag (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL
);

CREATE TABLE track_tag (
  track_id TEXT NOT NULL,
  tag_id INTEGER NOT NULL,
  PRIMARY KEY (track_id, tag_id),
  FOREIGN KEY (track_id) REFERENCES track(id) ON DELETE CASCADE,
  FOREIGN KEY (tag_id) REFERENCES tag(id) ON DELETE CASCADE
);

CREATE INDEX idx_track_tag_tag_id ON track_tag(tag_id);
CREATE INDEX idx_track_tag_track_id ON track_tag(track_id);
~~~

Preferred DTO shape:

~~~ts
tags: { name: string; display_name: string }[]
~~~

## Right Version

- Do not store `right_version` as a DB tag for now.
- Derive it from title/filename using a shared helper.
- Backend helper should answer whether title or filename contains a right-version marker.
- Frontend helper should use API-provided title/filename and color matching rows.
- If title is edited and no longer contains `(Right version)`, the special color should disappear.

## AI Cover

- Store `AI cover` as tag state via `tags`.
- Also derive AI-cover visual styling from title/filename marker.
- Backend helper should answer whether title or filename contains an AI-cover marker.
- Frontend helper should use API-provided title/filename and color matching rows.
- If title is edited and no longer contains `(AI cover)`, marker-based styling should disappear.
- Prefer category checkbox as source of truth for filtering.

## Download Flow

Files:

- `backend/src/downloads.rs`
- `backend/src/gemini.rs`
- `frontend/src/lib/components/domain/DownloadManager.svelte`
- `frontend/src/lib/components/domain/TrackMetadataModal.svelte`
- `frontend/src/lib/components/domain/TrackMetadataForm.svelte`
- `frontend/src/lib/components/domain/trackMetadataForm.ts`
- `frontend/src/lib/services/apiClient.ts`

Requirements:

- Metadata fetch should suggest tags: `gachi` and/or `ai-cover`.
- AI/Gemini may infer `Gachi` and `AI cover`, but keep it conservative.
- Review modal must show `Gachi` and `AI cover` checkboxes.
- Checkbox state is sent in confirm payload.
- Confirm download stores tags in DB.
- Filename/title normalization must convert right-version variants to `(Right version)`.
- Filename/title normalization must convert AI-cover variants to `(AI cover)`.

## Edit Flow

Files:

- `backend/src/models.rs`
- `backend/src/tracks.rs`
- `frontend/src/lib/components/domain/TrackMetadataModal.svelte`
- `frontend/src/lib/components/domain/TrackMetadataForm.svelte`
- `frontend/src/lib/components/domain/trackMetadataForm.ts`
- `frontend/src/lib/services/apiClient.ts`

Requirements:

- `PATCH /api/v1/tracks/:id` accepts `tags`.
- Edit modal initializes checkbox from track state.
- Changing only category checkboxes should update DB without rewriting audio file.
- Title/artist/artwork changes keep existing behavior.
- Rename still uses normalized category markers.

## Scanner

Files:

- `backend/src/scanner.rs`
- `backend/src/media.rs`
- `backend/src/util.rs`

Requirements:

- New files should be tagged `gachi` and/or `ai-cover` when detection says yes.
- Detection can use path/title/artist heuristics, but avoid broad false positives.
- Existing DB tags should not be removed just because a later scan is uncertain.
- If a file is moved/renamed and matched to an existing track, preserve tags.

## API

Add category support:

- `GET /api/v1/tracks?category=gachi` returns Gachi tracks.
- `GET /api/v1/tracks?category=ai-cover` returns AI-cover tracks.
- `GET /api/v1/categories` returns counts.

Expected category response:

~~~json
[
  { "name": "gachi", "display_name": "Gachi", "count": 12 },
  { "name": "ai-cover", "display_name": "AI cover", "count": 4 }
]
~~~

Return only categories with count greater than `0`.
Count queries should use `track_tag` indexes.

## Frontend UI

Files to inspect first:

- `frontend/src/lib/components/layout/RightSidebar.svelte`
- `frontend/src/lib/components/domain/TrackList.svelte`
- `frontend/src/lib/components/domain/TrackItem.svelte`
- `frontend/src/lib/stores/trackStore.ts`

Requirements:

- Add `Category` after `Artists`.
- Show `Gachi (count)` under `Category`.
- Show `AI cover (count)` under `Category`.
- Do not show `Category` if both counts are `0`.
- Do not show a category row if its count is `0`.
- Clicking `Gachi` filters track list to Gachi tracks.
- Clicking `AI cover` filters track list to AI-cover tracks.
- Track navigation should respect the active category filter.
- Right-version and AI-cover tracks should be visually distinct.
- Remove the `Controls` div/section.

## Tests

Backend:

- Migration adds `tag`, `track_tag`, and indexes.
- Track DTO includes `tags`.
- `PATCH` can update only tags.
- `GET /tracks?category=gachi` filters correctly.
- `GET /tracks?category=ai-cover` filters correctly.
- `GET /categories` returns only non-zero category counts.
- Right-version normalization handles variants.
- AI-cover normalization handles variants.

Frontend:

- Metadata modal shows and submits `Gachi` and `AI cover` checkboxes.
- Edit modal initializes checkbox from track.
- Category buttons filter tracks.
- Empty categories are hidden.
- Category section is hidden when all categories are empty.
- Right-version and AI-cover row styling appears for normalized title/filename.
- Controls section is gone.

## Verification

- Run `make back-test`.
- Run `make front-test`.
- Run `make ci` if practical.
- If any target is skipped, state why.
