# Roadmap

## Notes on the paper side

## Phase 1

- [x] load PlayerState fixes to use saved progress. Check what backend gets when we save state on closed tab.
- [x] Eliminate player.currentTime and only use audio.currentTime. Also, check other things duplicated and only use audio. That will prevent a lot of effects.
- [x] Rename playerStore to playlistStore. Move is_repeat logic to audioService.
- [x] Fix frontend
- [x] move next track out of audioService, think how. Maybe thats good case for effect. This effect has to change web title!
- [x] Move all dependencies to devDependencies. Eliminate usage of xior and use fetch
- [x] Try to go with https://github.com/unplugin/unplugin-icons - use lucide there
- [x] Bug - after loading page, if we select another track, it will not play. Check it.
- [x] Remove time since added from trackitem and also remove date-fns
- [x] Why we have +page.svelte and +layout.svelte? Should not that be only one?
- [x] once again fix saving state, log on backend, check close tab and other cases
- [x] Analyze all css methods which update several states in once. Make them with different methods. Remove all $: and refactor to svelte 5.
- [ ] ~~Divide frontend into components: footer / sidebar / tracklist, etc.~~
- [x] Style progress bar so it would be equal like tracklist' one. Remove styling from TrackItem.svelte.
- [x] Move to vscode, update workflow, aliases. Adapt this workflow. Remove cursorrules. Update all snippets. https://www.chatprd.ai/resources/PRD-for-Cursor - browsermcp.io
- [x] Implement and display per-track buffered time ranges using a new BufferedRangesService and update Slider.svelte to render these ranges.
- [x] css to fix too large music files names
- [x] ai workflow Tasks - use just task.md everywhere. Use single backticks. There has to be no step like "review" or "ci" or something.
- [x] Fixed position of buttons in player footer
- [x] Larger player footer. Move volume to the right side of next button.
- [x] Fix mobile footer css
- [x] Hover on play/next/prev/any buttons - I want to glow with blue, not like now
- [x] Mobile API for Safari - PWA + service worker
- [x] show history of tracks in right panel to check functionality of shuffle/repeat tracks - check it
- [x] Shuffle - playlist doesnt work correctly.
- [x] Run through prompt about enhancing project, get TODOs done
- [x] Remove test toasts notifications from frontend and backend
- [x] Test toast on track adding
- [x] Make docker compose working and also dockerize production dockerfile
- [x] Secret endpoint check!
- [x] docker setup for production check locally - will require Svelte static build - will require to remove SSR or to do something - refactoring and good code organizing needed
- [x] robots.txt, llm, so on
- [x] Get through all the mess. There is frontend/backend communication problem including production over SSR/browser localhost ports ... - analyze all places to fix. Also track covers webp.
- [x] Auth problem with cross-site cookies. Migrate to JWT and auth header?
- [x] Database, covers, etc. outside of /app directory in docker containers. Maybe /app_data/... After that change dockerignore.
- [x] Merge branch, deploy
- [x] Little bug - if we drag volume slider overflow left, it will show 100%. Also change cursor on dragging - on hover. Also have background dark colored for it.
- [x] Clicking on album image on player footer should scroll to this track in tracklist.
- [x] Scroll to track only if it is not visible on screen. Be sure it scrolls on first page load.
- [x] no cover album img - make it beautiful
- [x] QR code when SECRET_KEY exists
- [x] Test: redirect after login -> goes to localhost:8000 for production
- [x] When <1000px left/right padding on trackitem list has to be gone i think

## Phase 2: Backend Event-Driven Redesign

- [x] Swipe to open right sidebar
- [x] Make good layout for Mobile S sizes (320px)
- [x] Hide tests warning in backend
- [x] Throw out authentication from app, think of something nginx based yet easy to login in mobile
- [x] Fix PWA app and Music API for Safari iOS so it would be next/prev buttons, not seeks
- [x] no-cover.svg has to have cache policy
- [x] Recolor login.html
- [x] Full update of project. Fuck with bits-ui/sliders. also backend
- [x] Production dockerfile rewrite to use uv image + check
- [x] node 24 + python 3.13
- [x] Production problems - doesnt redirect correctly on token. Login on mobile doesnt get nice link.
- [x] sliders - beginning a bit filled; end not filled a bit
- [x] **Foundation: Database & Queues**
    - [x] Enhance `Track` schema with `inode` and `content_hash` for robust file tracking.
        - [x] Add `processing_status: str` enum (`PENDING`, `METADATA_DONE`, `ART_DONE`, `COMPLETE`, `ERROR`).
        - [x] Add `last_error_message: str | None`.
    - [x] Tune SQLite for concurrency by enabling WAL mode and a busy timeout. Maybe other tuning as well
        - [x] Enable `PRAGMA journal_mode=WAL` and `PRAGMA synchronous = NORMAL` on all connections.
        - [x] Set `PRAGMA busy_timeout` to 5000ms.
    - [x] Set up a task queue system (e.g., RQ) using DragonflyDB as the broker. Be sure you do it fine inside production image.
        - [x] Add `dragonfly` service to `docker-compose.yml` for local development.
        - [x] Add DragonflyDB installation to the production Dockerfile.
        - [x] Add `[program:dragonfly]` and `[program:rq-worker]` to `supervisord.conf`.
        - [x] Use two queues: `high_priority` for file events and `low_priority` for analysis (covers, ffprobe, etc.). Those will be just two tasks. Analysis will be one file but with functionality from different files.
- [x] **Core Processing Pipeline**
    - [x] Implement `watchdog` to monitor the music directory for real-time file changes (`created`, `modified`, `deleted`, `moved`).
    - [x] Remove all current code about processing files. We will start from scratch!!
    - [x] On startup, perform an initial full scan that populates the database or the queue before the watcher takes over.
        - [x] This scan will synchronously extract fast metadata (`mutagen`) in batches and save to the DB. It will then enqueue slower tasks (covers, duration) to the `low_priority` queue.
    - [x] Create an asynchronous worker to process `created` and `modified` events for metadata extraction and accurate duration analysis (using FFprobe).
    - [x] Create a dedicated worker for cover art extraction and processing, triggered after successful metadata processing.
    - [x] Create a dedicated worker to handle `deleted` events, ensuring tracks and their associated covers are removed.
    - [x] Implement logic to handle `moved` events by updating the file path based on inode, preserving the track's identity.
- [x] **Features & Robustness**
    - [x] Implement `TrackHistory` table and API to track the last 5 metadata changes per track and allow for rollbacks.
    - [x] Implement basic performance monitoring (e.g., queue depths).
- [o] ~~Analyze saving state - would it be faster and less simple code to just send state every second if it is changed?~~
- [x] Frontend should get only usable fields for /tracks.
- [x] Slider cursor hand on cover
- [x] robots.txt unauthorized + maybe other assets
- [x] rate limiting changes
- [x] start some e2e with production dockerimages
- [x] vulture/bandit linters
- [ ] Edit files and the tags in place
  - [ ] First of all: we should check on app load if we actually can change files (user might use read-only volume). Find the best way to find this. If we cant, we should disable all editing functionality.
  - [ ] Use modal window. Mobile - most of screen. Desktop idk. If there are no changes, click outside of modal will close it. If there are changes, we should confirm that. Also there has to be cancel button.
  - [ ] UI Trigger - three-dot menu on each TrackItem that reveals an "Edit" option. This keeps the main track list UI clean.
  - [ ] API Endpoint Design: A logical RESTful approach would be a PATCH request to an endpoint like /api/v1/tracks/{track_id}. The request body would contain the new metadata and the rename_file boolean flag. I want to send ONLY CHANGES, not all metadata. That way will be easier to save history. We have to handle the case when there is no changes - think what should we do because PATCH is not idempotent and we do not want to save that in history.
  - [ ] Tags have to have good encoding - ID3v2.4 with UTF-8 encoding (or ID3v2.3?)
  - [ ] Tags have to be windows compatible for filenames (prevent <, >, :, ", /, \, |, ?, *). Remove them.
  - [ ] Default option to rename filename as well (checkbox). Ticked by default, but user can prevent that. Use artist - track name notation. Always preview new filename. Make warning on long filenames I think (>100). Prevent to save 255 characters.
  - [ ] Save file history
  - [ ] Add reverting functionality
  - [ ] We have to think that in future we will have possibility to have Artist entity and maybe in future we will match whats in db with new file
  - [ ] Think already what to do with multiple artists - where do additional ones have to be stated and what will be the separator to divide them and show later. In Artist field metatags lets split them with ";". In filename lets split them with "ft. %artist%".
  - [ ] Cover art - cant be changed now, yet show it. In future I want to be able to fetch arts from some search, yt, or else and pick best one
  - [ ] Error handling on file save - if file is missing (rare case) or there are rights problem (common case probably) - show error to user
  - [ ] History has to save the exact change in jsonb and all file metadata with its filename in jsonb.
  - [ ] Show exact changes will be done - for example if we change encoding it also has to be shown
  - [ ] Make sure this would use less effects as possible. We have to include file_path now for /tracks still for editing files
  - [ ] There have to be fields with multiple artists to save later
- [ ] Remove files in editing file dialog - with confirmation.
- [ ] If user moves file on opened page, we should upload it to the server, but first show the dialog with filename and tags
- [ ] Mobile bug: right sidebar doesnt open on swipe
- [ ] Desktop bug: if user drags track and releases finger outside of slider on track, track will stop (fix the event, use mousedown/mouseup everythere)
- [ ] If we are selecting text on track item, it should not stop track. Btw think to edit the copied things to like artist - title.
- [ ] On hover on controls we should set hand cursor
- [ ] Make scrolling if only outside of page. Its kinda not fine now. Think about best UX first. Do not break on load page scrolling. Its like works on a 2nd try
- [ ] Too big DOM. Try to use `svelte-virtual`. Will it conflict with scroll into view or shuffle?
- [o] ~~minify options https://github.com/ntsd/sveltekit-html-minifier https://svelte.dev/docs/kit/migrating#Integrations-HTML-minifier~~
- [o] ~~Celery and async tasks~~
- [ ] e2e in CI before deployment after linters. Complex github actions flow.
- [ ] Complex e2e test: some file has to be flac with cover and wrong metadata for duration. Check metadata and cover works. Set added_at.
- [ ] Once again test all $effects, maybe too much of them. Delete console logs. isUserDragging needed?
- [ ] Get rid of SQLModel, only sqlalchemy. Remove all warnings disabling. remove all # noqa: F401
- [ ] Full test

## Phase 3

- [ ] docker-compose - i think we dont need separated volumes for cover/db, might be single
- [ ] production image nginx better logging. Logging across app.
- [ ] Hotkeys for player controls
- [ ] Hover player controls should show what will be done + also hotkey for that
- [ ] Marquee for long texts - all places
- [ ] ~~styling for playing music - make it less colored but on hover make blue colored styling for slider~~
- [ ] Get rid of fucking SSR and simplify code A LOT - ???.
- [ ] Render play button from tracklist under album cover
- [ ] Edit cover arts - download from some search, from youtube or else to pick one. I do not want to handle uploading covers yet I think.
- [ ] Define Artist entity
- [ ] Preview of each file - possibility to set title artist (choose?)
- [ ] Parse artists, make them unique, add to db. Make functionality to set artist for track. Remove artist from db if no tracks with this artist. Multiple artists for track.
- [ ] Artist page with all their tracks
- [ ] Artist can have many similar names (Тату = t.A.T.u.) - get from internet their possible titles for automatical matching later? AI?
- [ ] Write all markdown files with AI

## Phase 4

- [ ] Download page - with form to add url
- [ ] yt-dlp from yt (list domains)
- [ ] ...?? soundcloud?
- [ ] yt-dlp from other sites?

## Phase 5 - playlist management and album management

- [ ] Define Album entity, album page with tracks
- [ ] Define Playlist entity
- [ ] Implement playlist management service (Deferred)
- [ ] Many-to-many relation between tracks/albums/artists
- [ ] ~~Buffering - I think in real world it works super fine, no need.~~
  - [ ] ~~Should I force buffering of track?~~
  - [ ] ~~Should I buffer next/previous tracks for 3s? For that we have to have prev/nextTrackIndex (maybe on buttons?). prev/next track buttons should use them and unified function~~
- [ ] fast search - has to be server side to look vk/yt - and download!

## Phase 6 - vk

- [ ] vk.com player with download functionality
