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
- [x] Recolor all frontend. Switched on repeat/reshuffle buttons should be blue (check)
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
- [ ] Run through prompt about enhancing project, get TODOs done
- [ ] Remove test toasts notifications from frontend and backend
- [ ] Test toast on track adding
- [ ] Make docker compose working and also dockerize production dockerfile + start some e2e
- [ ] Rewrite all markdown files with AI
- [ ] Secret endpoint
- [ ] docker setup
- [ ] Publish
- [ ] Little bug - if we drag volume slider overflow left, it will show 100%. Also change cursor on dragging - on hover
- [ ] Clicking on album image on player footer should scroll to this track in tracklist.
- [ ] Analyze saving state - would it be faster and less simple code to just send state every second if it is changed?
- [ ] no cover album img - make it beautiful
- [ ] Render play button from tracklist under album cover
- [ ] Marquee for long texts
- [ ] Scroll to track only if it is not visible on screen. Be sure it scrolls on first page load.

## Phase 2

- [ ] Hotkeys for player controls
- [ ] Hover player controls should show what will be done + also hotkey for that
- [ ] Edit files in place. Normalize tags has to be automatical. Edit filename (windows names)
- [ ] If user moves file on opened page, we should upload it to the server
- [ ] Preview of each file - possibility to set title artist (choose?)
- [ ] Possibility to delete tracks
- [ ] History of file editing. Revert functionality.

## Phase 3

- [ ] Define Artist entity
- [ ] Parse artists, make them unique, add to db. Make functionality to set artist for track. Remove artist from db if no tracks with this artist. Multiple artists for track.
- [ ] Artist page with all their tracks
- [ ] Artist can have many similar names (Тату = t.A.T.u.) - get from internet their possible titles for automatical matching later? AI?

## Phase 4

- [ ] Implement playlist management service (Deferred)
- [ ] fast search - has to be server side
- [ ] Download page - with form to add url
- [ ] yt-dlp from yt (list domains)
- [ ] ...?? soundcloud?
- [ ] yt-dlp from other sites?

## Phase 5 - playlist management and album management

- [ ] Define Album entity, album page with tracks
- [ ] Define Playlist entity
- [ ] Many-to-many relation between tracks/albums/artists
- [ ] Should I force buffering of track?
- [ ] Should I buffer next/previous tracks for 3s? For that we have to have prev/nextTrackIndex (maybe on buttons?). prev/next track buttons should use them and unified function

## Phase 6 - vk

- [ ] vk.com player with download functionality
