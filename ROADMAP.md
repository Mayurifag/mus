# Implementation Plan

## TODO right now

why we need to set trackLoaded = true in handleSeeked? remove that


  function handleSeeked() {
    // Now that seeking is complete, we can safely set trackLoaded
    trackLoaded = true;
  }

why we need to clear shouldAutoPlay? and this code?

  // If we should auto-play (either from store state or saved intent), start playback
  if ($playerStore.isPlaying || shouldAutoPlay) {
    shouldAutoPlay = false; // Clear the flag
    audio.play().catch((error) => {
      console.error("Error playing audio after metadata loaded:", error);
      // Don't pause on AbortError - it's expected when changing tracks
      if (error.name !== "AbortError") {
        playerStore.pause();
      }
    });
  }

## Phase 1

- [ ] debouncedSavePlayerState fixes to use saved progress. Check what backend gets when we save state on closed tab.
- [ ] Analyze all css methods which update several states in once. Make them with different methods. Remove all $: and refactor to svelte 5.
- [ ] Recolor all frontend
- [ ] Larger player footer. Move volume to the right side of next button. Style progress bar so it would be equal like tracklist' one
- [ ] Progress bars have to show how much data cached already.
- [ ] Fix mobile footer css
- [ ] Adapt this workflow https://www.chatprd.ai/resources/PRD-for-Cursor
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
