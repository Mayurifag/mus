# Fix iOS PWA Media Session Controls

## User Problem
On Safari for iOS, the lock screen and control center media controls for the PWA show "seek forward" and "seek backward" buttons instead of the expected "next track" and "previous track" buttons. This provides a poor user experience for a music streaming application.

## High-Level Solution
The `AudioService` will be modified to correctly configure the Media Session API for iOS. The action handlers will be set only after audio playback has actively started, triggered by the `playing` event on the HTML audio element. The `seekforward` and `seekbackward` actions will be explicitly disabled by setting their handlers to `null`.

## Success Metrics
- On Safari for iOS, the media session controls display "next track" and "previous track" buttons.
- The next/previous track buttons successfully trigger the `trackStore.nextTrack()` and `trackStore.previousTrack()` functions.
- The standard play, pause, and seek bar controls continue to function correctly across all platforms.
- `make ci` passes without errors.

## Detailed Description
The core of the issue lies in when and how the `navigator.mediaSession.setActionHandler` calls are made. On Safari for iOS, these handlers must be set when audio is actively playing to ensure the correct controls are displayed.

The required changes are confined to `frontend/src/lib/services/AudioService.ts`. The current `setupMediaSession` method, which is called in the constructor, will be refactored. Its logic will be moved into a new method that is invoked by the audio element's `playing` event. This ensures the handlers are registered at the appropriate time. Additionally, we will explicitly disable the seek controls that iOS defaults to if the handlers are not null.

## Subtasks

### [ ] 1. Refactor Media Session Handler Initialization in AudioService
**Description**: Move Media Session API `setActionHandler` calls to be triggered by the `playing` event and explicitly disable seek controls.
**Details**:
In `frontend/src/lib/services/AudioService.ts`:
1.  Create a new private method named `setupActionHandlers`.
2.  Move all the logic (all `navigator.mediaSession.setActionHandler` calls) from the existing `setupMediaSession` method into the new `setupActionHandlers` method.
3.  In the `setupActionHandlers` method, add two new calls to explicitly disable the seek controls:
    - `navigator.mediaSession.setActionHandler("seekforward", null);`
    - `navigator.mediaSession.setActionHandler("seekbackward", null);`
4.  In the `setupEventListeners` method, add a new listener for the `playing` event that calls the new `setupActionHandlers` method: `this.audio.addEventListener("playing", this.setupActionHandlers);`.
5.  Remove the original `setupMediaSession` method entirely.
6.  Remove the call to `this.setupMediaSession()` from the `constructor`.
**Filepaths to Modify**: `frontend/src/lib/services/AudioService.ts`
**Relevant Make Commands (Optional)**: `make front-lint, make front-test`