import type { TrackState } from "$lib/stores/trackStore";
import { parseArtists } from "$lib/utils/formatters";
import { trackCategoryNames } from "$lib/utils/categoryMarkers";

export function selectedTrackIndexes(state: TrackState): number[] {
  return state.tracks.reduce<number[]>((indexes, track, index) => {
    if (
      (!state.selectedArtist ||
        parseArtists(track.artist).includes(state.selectedArtist)) &&
      (!state.selectedCategory ||
        trackCategoryNames(track).includes(state.selectedCategory))
    ) {
      indexes.push(index);
    }
    return indexes;
  }, []);
}

export function navigateFilteredTracks(
  state: TrackState,
  indexes: number[],
  direction: 1 | -1,
): Partial<TrackState> {
  const scopedIndexById = new Map(
    indexes.map((index) => [state.tracks[index].id, index]),
  );
  if (state.is_shuffle) {
    for (
      let position = state.historyPosition + direction;
      position >= 0 && position < state.playHistoryIds.length;
      position += direction
    ) {
      const index = scopedIndexById.get(state.playHistoryIds[position]);
      if (index !== undefined) {
        return {
          currentTrackId: state.tracks[index].id,
          historyPosition: position,
        };
      }
    }
  }

  const currentTrackIndex = state.tracks.findIndex(
    (track) => track.id === state.currentTrackId,
  );
  const currentInScope = indexes.includes(currentTrackIndex);
  let index: number;
  if (state.is_shuffle && direction === 1) {
    const available =
      currentInScope && indexes.length > 1
        ? indexes.filter((index) => index !== currentTrackIndex)
        : indexes;
    index = available[Math.floor(Math.random() * available.length)];
  } else {
    const position =
      state.currentTrackId === null ? -1 : indexes.indexOf(currentTrackIndex);
    const nextPosition =
      position === -1
        ? direction === 1
          ? 0
          : indexes.length - 1
        : (position + direction + indexes.length) % indexes.length;
    index = indexes[nextPosition];
  }
  const track = state.tracks[index];
  if (!state.is_shuffle) {
    return { currentTrackId: track.id };
  }
  if (direction === 1 && currentInScope && indexes.length === 1) return {};

  const history = state.playHistoryIds
    .slice(
      0,
      direction === 1 ? Math.max(state.historyPosition + 1, 0) : undefined,
    )
    .filter((id) => scopedIndexById.has(id));
  if (history.length === 0 && currentInScope && state.currentTrackId) {
    history.push(state.currentTrackId);
  }
  const playHistory =
    direction === 1 ? [...history, track.id] : [track.id, ...history];
  return {
    currentTrackId: track.id,
    playHistoryIds: playHistory,
    historyPosition: direction === 1 ? playHistory.length - 1 : 0,
  };
}
