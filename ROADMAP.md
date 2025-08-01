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
- [x] Divide frontend into components: footer / sidebar / tracklist, etc.
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
- [x] Edit files and the tags in place
  - [x] Return file_path to /tracks endpoint because frontend needs that data
  - [x] TrackHistory has to be added with missing fields. I want there history changes and first will be the scan init. Initial data saved without delta
  - [x] First of all: we should check on app load if we actually can change files (user might use read-only volume). Find the best way to find this. If we cant, we should disable all editing functionality.
  - [x] Use modal window. Mobile - most of screen. Desktop idk. If there are no changes, click outside of modal will close it. If there are changes, we should confirm that. Also there has to be cancel button.
  - [x] UI Trigger - three-dot menu on each TrackItem that reveals an "Edit" option. This keeps the main track list UI clean.
  - [x] API Endpoint Design: A logical RESTful approach would be a PATCH request to an endpoint like /api/v1/tracks/{track_id}. The request body would contain the new metadata and the rename_file boolean flag. I want to send ONLY CHANGES, not all metadata. That way will be easier to save history. We have to handle the case when there is no changes - think what should we do because PATCH is not idempotent and we do not want to save that in history.
  - [x] Tags have to have good encoding - ID3v2.4 with UTF-8 encoding (or ID3v2.3?)
  - [x] Tags have to be windows compatible for filenames (prevent <, >, :, ", /, \, |, ?, *). Remove them.
  - [x] Default option to rename filename as well (checkbox). Ticked by default, but user can prevent that. Use artist - track name notation. Always preview new filename. Make warning on long filenames I think (>100). Prevent to save 255 characters.
  - [x] Save file history
  - [ ] ~~Add reverting functionality~~
  - [x] We have to think that in future we will have possibility to have Artist entity and maybe in future we will match whats in db with new file
  - [x] Think already what to do with multiple artists - where do additional ones have to be stated and what will be the separator to divide them and show later. In Artist field metatags lets split them with ";". In filename lets split them with "ft. %artist%".
  - [x] Cover art - cant be changed now, yet show it. In future I want to be able to fetch arts from some search, yt, or else and pick best one
  - [x] Error handling on file save - if file is missing (rare case) or there are rights problem (common case probably) - show error to user
  - [x] History has to save the exact change in jsonb and all file metadata with its filename in jsonb.
  - [x] Make sure this would use less effects as possible. We have to include file_path now for /tracks still for editing files
  - [x] There have to be fields with multiple artists to save later
  - [x] User cant click "Save" if there are no changes and no tags edits needed. API should do nothing on no changes.
  - [x] ~~Lets use PlayerState for editing files boolean to not add system permissions endpoint~~
  - [x] I want final filename for tracks to be saved like "Artist 1, Artist 2 - Title" (no using featuring word or else)
- [x] ~~ghostty and~~ mise in bashrc
- [x] Edit button on dropdown menu on hover restyling with little glow instead of blue background
- [x] 0 changes move to the left in edit modal
- [x] Additional artists styling on frontend and inside editing modal. Also editing modal - original cover to the left, edit in place. maybe filename shown with little font somewhere.
- [x] "Edit" -> make not a dropdown menu but an icon with tooltip. Hide on small screens.
- [x] Mobile bug: right sidebar doesnt open on swipe
- [x] Desktop bug: if user drags track and releases finger outside of slider on track, track will stop (fix the event, use mousedown/mouseup everythere)
- [x] If we are selecting text on track item, it should not stop track. Btw think to edit the copied things to like artist - title.
- [x] On hover on controls we should set hand cursor
- [x] Panel of last changes. Rename. Think of icons. It seems works wrong now the change itself saved
- [x] Too big DOM
  - [x] When we add track or delete it, event has to not reload all tracks, it has to add it/remove it from the list, just change dom a little bit. Same with edit. Do not ever reload all list.
  - [x] Delete all current code to scroll to current track or another automatic scrolling
  - [x] Add <https://github.com/inokawa/virtua>. Use context7 documentation to get latest information to work it with svelte
  - [x] Make scrolling - on page load
  - [x] Test if scrolling works all scenarios
  - [x] Will CTRL+F work? If not - we need simple search or smth - needs to be checked
  - [x] Will shuffle work? - seems yes
  - [x] After edit track we have to scroll to the same place where scrolling was
  - [o] ~~During testing check console log for effects~~
- [x] Fix progress clicking on slider sometimes

## Phase debugging, make whistles and blows

- [x] Effects refactoring
  - [x] Use app a little bit and see if there are much effects used to prevent cascade of them. Are there any ready solutions for that?
  - [x] derived.by - maybe use gemini to see what can be done better
  - [x] some util log
  - [x] 2 effects on filling slider - maybe it fills both sliders so 2 effects?
  - [x] Embrace $derived to make your components more readable, maintainable, and performant.
  - [x] Refactor side effects ($effect) to be more focused and to implement more robust application logic (like the player state saving).
- [o] ~~Add rq-dashboard~~
- [x] Add watchdog events monitoring
  - [x] It seems, app changes are triggering watchdog - not sure! App changes should not update mtime! It updates now
- [x] init scan - doesnt update covers
- [x] Mobile - make footer upper. PWA instructions in QR code add. Process anything not working.
- [x] service worker broken /images/no-cover.svg for production
- [x] token login should not redirect and contain link as is - to be saved for pwa - fix nginx conf. For PWA - check image for app added, I want icon. Other params?
- [x] fix long token visual - make it with x-scrolling in qr modal
- [o] ~~nginx.conf - wrong redirect. I think that just has to reload page so app will load so no changes to URL~~
- [x] If user moves file on opened page, we should upload it to the server, but first show the dialog with filename and tags
- [x] Codereview drag and drop feature:
  - [x] frontendCoverExtractor.ts is total bullshit now
  - [x] Rename apiErrorHandler and see if we can reuse some code from there anywhere else for api frontend calls - fine if not
  - [x] too much comments/descriptions (backend/src/mus/util/filename_utils.py etc)
  - [x] V1_TO_EASY_MAPPING wtf is this. Do I need it? Can we just ignore nonv2 tags? We can use newer ones later
  - [x] all effects have been logged?
  - [x] Layout_TrackChangeHandler - executes twice - why?
  - [x] check effects + check draganddrop
- [x] Edit track functionality enhancements
  - [ ] ~~Show exact changes will be done - for example if we change encoding it also has to be shown~~
  - [ ] ~~Check that wrong tags could be fixed in UI - wrong encoding, wrong fields filled~~
  - [x] check long filenames on edit - add warning
  - [x] Remove files in editing file dialog - with confirmation. It may be not files but just entries in db.
  - [x] Ignore all tags on creation except author/trackname. We will use it and/or overwrite just them. Others will be ignored (not deleted).
- [x] Change mp3tag.js to something else. It doesnt support typescript + i might need something else.. hopefully installing in devdependencies..
  - [x] Full tests cover on audioFileAnalyzer.ts first!
  - [x] refactor with another library - i do not think we need all those fields. Maybe vibecode.
- [x] Deploy to GHCR in addition to Docker Hub to avoid Watchtower authentication issues
  - [x] Update GitHub Actions workflow to push to both registries
  - [x] Update production deployment to use GHCR instead of Docker Hub
- [x] fix album covers nginx config for things like production.com/api/v1/tracks/17/covers/small.webp?v=1753112348
- [x] On adding file on drag and drop - it produces too many events.. Should it? Maybe no need to produce events on file creation from upload?
- [x] Setup playwright mcp. Rewrite all AGENT_TASKS prompts with info about playwright mcp. Also if no tracks found - just tell that no sleep needed, its fine.
- [x] Work on snippets for LLM
  - [x] https://x.com/steipete/status/1940314756705132683
  - [x] https://x.com/robzolkos/status/1940462968593875060
  - [x] Update mr alias to include full text from snippet
- [o] ~~minify options https://github.com/ntsd/sveltekit-html-minifier https://svelte.dev/docs/kit/migrating#Integrations-HTML-minifier~~
- [x] ~~Celery and async tasks~~
- [x] Wipe out history changes completely I do not need them - at least now and they make code messy.
- [x] Events refactoring ideas - On app launch too many slow metadata going on.
  - [x] As a beginning I need to document for now how it works and how should it work. External/internal changes all types of events. Maybe some kind of flowchart -> in future to transform onto finite state machine
  - [x] Change from rq to arq and to async code here and there
  - [x] slow metadata - convert automatically to UTF-8 id2V2.3
  - [x] whats slowing there - i think we might do things faster
  - [x] We should have different code for when event is from EXTERNAL file changes and from app. That would be much easier to maintain.
  - [x] When initial scan is going on: we might have "loading covers" based on metadata not done status in db on frontend. We might fire events without notification to change files. We might go one by one in single task and fire event on each cover processing and other metadata. We still need to save the state to continue on failures - or just dont give a fuck about that? because we will each time just might select files without processed metadata.
  - [x] I do not really use processing_status'es. I should leave just 2 of them. Pending and done. Or anything else? might also have "error" status to have junkyard for files with errors to not reprocess them.
  - [x] Refactor slow metadata - have service with each step in its own service. It has to extract cover, process duration, change encoding and save to db that file is processed.
  - [o] ~~Is that a bad thing to show just really original image with real extension and so on? We will parse less~~
  - [x] Maximum parallelism (Use a ProcessPoolExecutor for CPU-Bound Code) - only applicable for first scan
  - [x] I have to refactor first for a single track and for batch
- [x] e2e test for each scenario and fix each task / frontend.
  - [x] current state: track deletion fires but does not change frontend in playwright environment (?)
  - [o] ~~we will have to write at least successful flow for each event~~
  - [x] Complex e2e test: some file has to be flac/wav with cover and wrong metadata for duration. Check metadata and cover works. Set added_at.
- [x] Get back files from backup
- [x] ffs refactor docker-compose.override.yml.example shared envs and things + add context for AI to also change non example file
- [o] ~~After recent changes e2e takes too long and fails on timeout~~
- [x] Container labels: add latest commit title, hash and build time
- [x] Too much healthchecks. I need it very rarely. Can we check things once and every 1h or so? its not critical anyway I think - or remove healthcheck at all because for what reason do I need it???
- [x] Broken covers
- [x] After cover fixes - reset branches and select only needed changes.
- [x] Install sqlite3 for debugs
- [x] After changes read only filesystem won't work. We have to fix it and use flags on readonly
  - [x] ~~seems permission check after startup! so it has to be before/after fast data and used for slow scan flag.~~
- [x] Recurring task with production - known bugs Desktop / PWA / iphone
  - [x] PWA - last tracks are not shown under player footer - maybe I have to delete prev "fix" of phones placing - just watch recent changes to find problematic code
  - [x] Fix PWA - it shows tracks under notch and so on. On the bottom it overlaps with ios bar to open recent apps

PWA - music nog going next automatically, switch from effect

## Phase non needed features

- [ ] Case to check in e2e: file edited externally like delete-and-create with new data. Will it be treated like delete and create? Will it be deleted and created with new id? will my save state thing work?
- [ ] on close tab did not restore track - bug. Maybe we have to reimplement. Maybe we have to save that in local storage and send once in a while. UDP - do not need to wait 200.
- [ ] Player footer desktop - on change windows calculate div for player controls - this will allow to have full size for artist-title
- [ ] e2e in CI before deployment after linters. Complex github actions flow.
- [ ] Make sure initial scan on startup is not blocking "healthy" status for backend docker container.
  - [ ] only fast one blocks. I have to refactor startup calls into its own non-blocking service. There will be fast/slow/watcher one by one launched
- [ ] more e2e scenarios
  - [ ] Duration - written in db and in track
- [ ] wtf is "track updated" event on slow metadata after create? Nothing wrong just bad naming
- [o] slow metadata on startup doesnt standartize id3 version and encoding + also doesnt save correct duration to file tags - this has to be single file save - so single job? I have to use different fields based on file for this length, so use library..
  - [x] Standartize on startup
  - [ ] Make it efficient with only 1 final change of file for both duration and standartize
- [ ] Slider has to be smaller by default and on hover it has to be bigger in size like now
- [ ] ~~Revert functionality UI~~
- [ ] Remove non-docker development - not sure if thats needed - actually needed because AI doesnt understand what env im working in currently. Less commands is better
- [ ] Get rid of SQLModel, only sqlalchemy. Remove all warnings disabling. remove all # noqa: F401 - actually think to move everything in redis so there will be no sql db. But - think of relationships such model. Redis might have relationships or something.. I mean we can give up some consistency..
- [ ] Sort tracks by different fields and ways
- [ ] Continue refactoring effects
- [ ] fast search - has to be server side to look vk/yt - and download in future!
- [ ] Download track functionality
- [ ] docker-compose - i think we dont need separated volumes for cover/db, might be single
- [ ] production image nginx better logging. Logging across app (!!!)
- [ ] Hotkeys for player controls
- [ ] Hover player controls should show what will be done + also hotkey for that
- [ ] Marquee for long texts - all places - not sure where we exactly have to do that. Probably for player footer
- [ ] ~~styling for playing music - make it less colored but on hover make blue colored styling for slider~~
- [ ] Get rid of fucking SSR and simplify code A LOT - ???.
- [ ] Render play button from tracklist under album cover in tracklist
- [ ] Edit cover arts - download from some search, from youtube or else to pick one. I do not want to handle uploading covers yet I think.
- [ ] Define Artist entity
- [ ] Edit modal - possibility to set title artist (choose?)
- [ ] Parse artists, make them unique, add to db. Make functionality to set artist for track. Remove artist from db if no tracks with this artist. Multiple artists for track.
- [ ] Artist page with all their tracks
- [ ] Artist can have many similar names (Тату = t.A.T.u.) - get from internet their possible titles for automatical matching later? AI?
- [ ] "Play next" functionality

## Phase yt

- [ ] Download page - with form to add url
- [ ] yt-dlp from yt (list domains)
- [ ] yt-dlp from other sites?

## Phase vk / other services ?

- [ ] vk.com player with download functionality
- [ ] ...?? soundcloud?

## Phase playlist management and album management

- [ ] Define Album entity, album page with tracks
- [ ] Define Playlist entity
- [ ] Implement playlist management service (Deferred)
- [ ] Many-to-many relation between tracks/albums/artists

## Ideas

- [ ] Every minute check if folder still read-only or writeable - this might be one of the little things nobody notices
