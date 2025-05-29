# Roadmap

## Notes on the paper side

## Phase 1

- [x] load PlayerState fixes to use saved progress. Check what backend gets when we save state on closed tab.
- [x] Eliminate player.currentTime and only use audio.currentTime. Also, check other things duplicated and only use audio. That will prevent a lot of effects.
- [x] Rename playerStore to playlistStore. Move is_repeat logic to audioService.
- [x] Fix frontend
- [ ] move next track out of audioService, think how. Maybe thats good case for effect. This effect has to change web title!
- [ ] Try to go with https://github.com/unplugin/unplugin-icons - use lucide there
- [ ] Move all dependencies to devDependencies. Eliminate usage of xior and use fetch
- [ ] Why we have +page.svelte and +layout.svelte? Should not that be only one?
- [ ] Analyze all css methods which update several states in once. Make them with different methods. Remove all $: and refactor to svelte 5.
- [ ] Recolor all frontend. Switched on repeat/reshuffle buttons should be blue (check)
- [ ] Divide frontend into components: footer / sidebar / tracklist, etc.
- [ ] Larger player footer. Move volume to the right side of next button.
- [ ] Style progress bar so it would be equal like tracklist' one. Remove styling from TrackItem.svelte. Progress bars have to show how much data cached already.
- [ ] Fix mobile footer css
- [ ] Move to vscode, update workflow, aliases. Adapt this workflow. Remove cursorrules. Update all snippets. https://www.chatprd.ai/resources/PRD-for-Cursor - browsermcp.io
- [ ] Mobile API for Safari - PWA + service worker
- [ ] show history of tracks in right panel to check functionality of shuffle/repeat tracks - check it
- [ ] Run through prompt about enhancing project, get TODOs done
- [ ] Make docker compose working and also dockerize production dockerfile + start some e2e
- [ ] Rewrite all markdown files with AI
- [ ] Publish

## Phase 2

- [ ] Hotkeys for player controls
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
- [ ] vk.com player with download functionality
- [ ] ...?? soundcloud?
- [ ] yt-dlp from other sites?

## Phase 5 - playlist management and album management

- [ ] Define Album entity, album page with tracks
- [ ] Define Playlist entity
- [ ] Many-to-many relation between tracks/albums/artists
