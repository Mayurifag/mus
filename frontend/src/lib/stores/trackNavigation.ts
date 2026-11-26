import type { TrackState } from "$lib/stores/trackStore";
import type { Track } from "$lib/types";

const indexesByLibrary = new WeakMap<readonly Track[], Map<string, number>>();

export function trackIndex(
  tracks: readonly Track[],
  id: string | null,
): number {
  if (id === null) return -1;
  let indexes = indexesByLibrary.get(tracks);
  if (!indexes) {
    indexes = new Map();
    for (const [index, track] of tracks.entries()) indexes.set(track.id, index);
    indexesByLibrary.set(tracks, indexes);
  }
  return indexes.get(id) ?? -1;
}

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

export const handleShuffleNext = (state: TrackState): Partial<TrackState> => {
  if (state.historyPosition < state.playHistoryIds.length - 1) {
    return moveInHistory(state, state.historyPosition + 1);
  }

  const current = trackIndex(state.tracks, state.currentTrackId);
  const available = state.tracks.length - Number(current !== -1);
  if (available === 0) return {};
  const offset = Math.floor(Math.random() * available);
  const selectedTrack =
    state.tracks[offset + Number(current !== -1 && offset >= current)];
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
  const { tracks, currentTrackId, playHistoryIds, historyPosition } = state;
  if (historyPosition > 0) return moveInHistory(state, historyPosition - 1);
  if (currentTrackId === null) return {};

  const previousIndex = calculatePreviousIndex(
    trackIndex(tracks, currentTrackId),
    tracks.length,
  );
  const id = tracks[previousIndex].id;
  const history =
    playHistoryIds.length === 0 ||
    (playHistoryIds.length === 1 && historyPosition === 0)
      ? [currentTrackId]
      : playHistoryIds;
  return {
    currentTrackId: id,
    playHistoryIds: [id, ...history],
    historyPosition: 0,
  };
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
