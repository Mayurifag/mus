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

- [ ] UX - make it usable (+ move controls on bottom)
- [x] Stream - not by filepath, but by id (a bit slower, yet less attack surface)
- [ ] UI - make it look nice
- [ ] When open page first time, should be selected first track to play paused 0 sec
- [ ] When we close tab and open again, should be paused from the same track same time.
- [ ] research cache possibilities - save to cache and clean

## Phase 6: Release MVP

- [x] Github Actions CI
- [x] Dockerized CD

## Phase 7: after MVP

- [ ] fast search - has to be server side
- [ ] Many-to-many relation between tracks/albums/artists
- [ ] Define Album entity
- [ ] album little images converting/saving/showing on
- [ ] Define Playlist entity
- [ ] Implement playlist management service (Deferred)

## Edit files in place

- [ ] Page to edit filename and tags. Also normalize tags has to be automatical.
- [ ] Possibility to delete tracks
- [ ] History of that - deleted tracks, history for each file, revert functionality

## Download functionality

- [ ] Download page - with form to add url
- [ ] yt-dlp from yt (list domains)
- [ ] vk.com player with download functionality
- [ ] ...?? soundcloud?
- [ ] yt-dlp from other sites?

## Load from user computer functionality

- [ ] If user moves file on opened page, we should upload it to the server
- [ ] Preview of each file - possibility to set title artist (choose?)

## Things nice to have

- [ ] Limit tracks by 5000 on page?
- [ ] Define Artist entity
- [ ] Artist page with all their tracks

## Notes
- Project management rules are defined in `.cursor/rules/`
- Use structured logging for all operations
- Follow hexagonal architecture principles
- Maintain async/await pattern throughout
- Album, Artist, and Playlist features are deferred to future phases
