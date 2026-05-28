import type { Track } from "$lib/types";
import type { TrackStoreState } from "$lib/stores/trackStore";

export const calculatePreviousIndex = (
  currentIndex: number,
  tracksLength: number,
): number => (currentIndex === 0 ? tracksLength - 1 : currentIndex - 1);

export const calculateNextIndex = (
  currentIndex: number,
  tracksLength: number,
): number => (currentIndex + 1) % tracksLength;

export const findTrackIndex = (tracks: Track[], targetTrack: Track): number =>
  tracks.findIndex((track) => track.id === targetTrack.id);

const moveInHistory = (
  state: TrackStoreState,
  newPosition: number,
): Partial<TrackStoreState> => {
  const targetTrack = state.playHistory[newPosition];
  const targetIndex = findTrackIndex(state.tracks, targetTrack);

  if (targetIndex === -1) return {};

  return {
    currentTrackIndex: targetIndex,
    currentTrack: targetTrack,
    historyPosition: newPosition,
  };
};

const handleSequentialPreviousWithHistory = (
  state: TrackStoreState,
): Partial<TrackStoreState> => {
  if (state.currentTrackIndex === null) return {};

  const previousIndex = calculatePreviousIndex(
    state.currentTrackIndex,
    state.tracks.length,
  );
  const previousTrack = state.tracks[previousIndex];

  return {
    currentTrackIndex: previousIndex,
    currentTrack: previousTrack,
    playHistory: [previousTrack, state.currentTrack!],
    historyPosition: 0,
  };
};

const extendHistoryBackward = (
  state: TrackStoreState,
): Partial<TrackStoreState> => {
  if (state.currentTrackIndex === null) return {};

  const previousIndex = calculatePreviousIndex(
    state.currentTrackIndex,
    state.tracks.length,
  );
  const previousTrack = state.tracks[previousIndex];

  return {
    currentTrackIndex: previousIndex,
    currentTrack: previousTrack,
    playHistory: [previousTrack, ...state.playHistory],
    historyPosition: 0,
  };
};

export const handleShuffleNext = (
  state: TrackStoreState,
): Partial<TrackStoreState> => {
  if (state.historyPosition < state.playHistory.length - 1) {
    return moveInHistory(state, state.historyPosition + 1);
  }

  const availableTracks = state.tracks.filter(
    (_, i) => i !== state.currentTrackIndex,
  );
  if (availableTracks.length === 0) return {};

  const randomIndex = Math.floor(Math.random() * availableTracks.length);
  const selectedTrack = availableTracks[randomIndex];
  const newIndex = findTrackIndex(state.tracks, selectedTrack);

  const newHistory = [...state.playHistory];

  if (state.currentTrackIndex !== null && state.historyPosition === -1) {
    newHistory.push(state.tracks[state.currentTrackIndex]);
  }

  newHistory.push(selectedTrack);

  return {
    currentTrackIndex: newIndex,
    currentTrack: selectedTrack,
    playHistory: newHistory,
    historyPosition: newHistory.length - 1,
  };
};

export const handleShufflePrevious = (
  state: TrackStoreState,
): Partial<TrackStoreState> => {
  if (state.historyPosition > 0) {
    return moveInHistory(state, state.historyPosition - 1);
  }

  const isMinimalHistory =
    state.playHistory.length === 0 ||
    (state.playHistory.length === 1 && state.historyPosition === 0);

  if (isMinimalHistory) {
    return handleSequentialPreviousWithHistory(state);
  }

  return extendHistoryBackward(state);
};

export const clearShuffleHistory = (
  state: TrackStoreState,
  newShuffleState: boolean,
): Partial<TrackStoreState> => {
  const isDisablingShuffle = state.is_shuffle && !newShuffleState;

  if (isDisablingShuffle && state.currentTrack) {
    return {
      is_shuffle: newShuffleState,
      playHistory: [state.currentTrack],
      historyPosition: 0,
    };
  }

  return { is_shuffle: newShuffleState };
};
