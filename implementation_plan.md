# Implementation Plan

## Phase 1: Core Setup
- [x] Project structure setup
- [x] Configuration files
- [x] Development environment
- [x] Basic logging setup
- [x] Docker configuration

## Phase 2: Domain Layer
- [x] Define Track entity
- [x] Implement track-related domain services

## Phase 3: Application Layer
- [x] Implement track scanning service
- [x] Implement track search service
- [x] Implement track metadata extraction service

## Phase 4: Infrastructure Layer
- [x] Implement SQLite repository for tracks
- [x] Implement file system scanner for tracks
- [x] Implement web API endpoints for tracks
- [x] Implement HTMX frontend for track listing
- [x] Implement audio streaming for tracks

## Phase 5: Good enough MVP

- [x] UX - make it usable (+ move controls on bottom)
- [x] Stream - not by filepath, but by id (a bit slower, yet less attack surface)
- [x] UI - make it look nice
- [x] When open page first time, should be selected first track to play paused 0 sec
- [x] When we close tab and open again, should be paused from the same track same time.
- [x] Refactor all css and whats applying by js:
  - [x] Common things like hidden/visible/etc. should be extracted into their own css classes and used in html in elements.
  - [x] Javascript has to be changed to apply classes, not hardcoding styles
  - [x] Fix css problem with next/prev track bug. Logic has not to go through all elements because thats shit performance.
- [ ] cache - current track, next. When click 'next' - remove -2 tracks from cache. Keep 3 tracks in cache. Make sure tracks are actually given chunk by chunk
- [x] album little images converting/saving/showing on
- [x] minimum volume bug - has to mute everything, doesnt mute

## Phase 6: Release MVP

- [ ] Github Actions CI fixes
- [ ] Dockerized CD

## Phase 7: after MVP

- [ ] Force rescan - make this button better!!
- [ ] fast search - has to be server side
- [ ] Run through prompt about enhancing project, get TODOs done
- [ ] Make /state sending through navigator.beacon. Its not THAT robust though and needs tests for my browser workflows.
- [ ] Load state should also scroll webpage so track to play will be centered
- [x] Add Stylelint for CSS linting and formatting
- [x] Add ESLint (standard config) for JS linting
- [ ] Define Album entity
- [ ] Many-to-many relation between tracks/albums/artists
- [ ] Define Playlist entity
- [ ] Implement playlist management service (Deferred)
- [ ] Move to scss. Redone css from scratch.

## Edit files in place and upload from user computer

- [ ] If user moves file on opened page, we should upload it to the server
- [ ] Preview of each file - possibility to set title artist (choose?)
- [ ] Page to edit filename and tags. Also normalize tags has to be automatical.
- [ ] Possibility to delete tracks
- [ ] History of that - deleted tracks, history for each file, revert functionality

## Download functionality

- [ ] Download page - with form to add url
- [ ] yt-dlp from yt (list domains)
- [ ] vk.com player with download functionality
- [ ] ...?? soundcloud?
- [ ] yt-dlp from other sites?

## Things nice to have

- [ ] Limit tracks by 5000 on page?
- [ ] Define Artist entity
- [ ] Artist page with all their tracks
- [ ] Artist can have many similar names (Тату = t.A.T.u.)
- [ ] Saving state through websocket

## Notes
- Project management rules are defined in `.cursor/rules/`
- Use structured logging for all operations
- Follow hexagonal architecture principles
- Maintain async/await pattern throughout
- Album, Artist, and Playlist features are deferred to future phases
