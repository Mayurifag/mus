import type { TrackState } from "$lib/stores/trackStore";

export const calculatePreviousIndex = (
  currentIndex: number,
  tracksLength: number,
): number => (currentIndex === 0 ? tracksLength - 1 : currentIndex - 1);

export const calculateNextIndex = (
  currentIndex: number,
  tracksLength: number,
): number => (currentIndex + 1) % tracksLength;

const moveInHistory = (
  state: TrackState,
  newPosition: number,
): Partial<TrackState> => {
  return {
    currentTrackId: state.playHistoryIds[newPosition],
    historyPosition: newPosition,
  };
};

const handleSequentialPreviousWithHistory = (
  state: TrackState,
): Partial<TrackState> => {
  if (state.currentTrackId === null) return {};

  const previousIndex = calculatePreviousIndex(
    state.tracks.findIndex((track) => track.id === state.currentTrackId),
    state.tracks.length,
  );
  const previousTrack = state.tracks[previousIndex];

  return {
    currentTrackId: previousTrack.id,
    playHistoryIds: [previousTrack.id, state.currentTrackId],
    historyPosition: 0,
  };
};

const extendHistoryBackward = (state: TrackState): Partial<TrackState> => {
  if (state.currentTrackId === null) return {};

  const previousIndex = calculatePreviousIndex(
    state.tracks.findIndex((track) => track.id === state.currentTrackId),
    state.tracks.length,
  );
  const previousTrack = state.tracks[previousIndex];

  return {
    currentTrackId: previousTrack.id,
    playHistoryIds: [previousTrack.id, ...state.playHistoryIds],
    historyPosition: 0,
  };
};

export const handleShuffleNext = (state: TrackState): Partial<TrackState> => {
  if (state.historyPosition < state.playHistoryIds.length - 1) {
    return moveInHistory(state, state.historyPosition + 1);
  }

  const availableTracks = state.tracks.filter(
    (track) => track.id !== state.currentTrackId,
  );
  if (availableTracks.length === 0) return {};

  const randomIndex = Math.floor(Math.random() * availableTracks.length);
  const selectedTrack = availableTracks[randomIndex];
  const newHistory = [...state.playHistoryIds];

  if (state.currentTrackId !== null && state.historyPosition === -1) {
    newHistory.push(state.currentTrackId);
  }

  newHistory.push(selectedTrack.id);

  return {
    currentTrackId: selectedTrack.id,
    playHistoryIds: newHistory,
    historyPosition: newHistory.length - 1,
  };
};

export const handleShufflePrevious = (
  state: TrackState,
): Partial<TrackState> => {
  if (state.historyPosition > 0) {
    return moveInHistory(state, state.historyPosition - 1);
  }

  const isMinimalHistory =
    state.playHistoryIds.length === 0 ||
    (state.playHistoryIds.length === 1 && state.historyPosition === 0);

  if (isMinimalHistory) {
    return handleSequentialPreviousWithHistory(state);
  }

  return extendHistoryBackward(state);
};

export const clearShuffleHistory = (
  state: TrackState,
  newShuffleState: boolean,
): Partial<TrackState> => {
  const isDisablingShuffle = state.is_shuffle && !newShuffleState;

  if (isDisablingShuffle && state.currentTrackId) {
    return {
      is_shuffle: newShuffleState,
      playHistoryIds: [state.currentTrackId],
      historyPosition: 0,
    };
  }

  return { is_shuffle: newShuffleState };
};
