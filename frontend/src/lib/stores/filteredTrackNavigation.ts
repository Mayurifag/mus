import type { TrackStoreState } from "$lib/stores/trackStore";
import { parseArtists } from "$lib/utils/formatters";
import { trackCategoryNames } from "$lib/utils/categoryMarkers";

export function selectedTrackIndexes(state: TrackStoreState): number[] {
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
  state: TrackStoreState,
  indexes: number[],
  direction: 1 | -1,
): Partial<TrackStoreState> {
  const scopedIndexById = new Map(
    indexes.map((index) => [state.tracks[index].id, index]),
  );
  if (state.is_shuffle) {
    for (
      let position = state.historyPosition + direction;
      position >= 0 && position < state.playHistory.length;
      position += direction
    ) {
      const index = scopedIndexById.get(state.playHistory[position].id);
      if (index !== undefined) {
        return {
          currentTrackIndex: index,
          currentTrack: state.tracks[index],
          historyPosition: position,
        };
      }
    }
  }

  const currentInScope =
    state.currentTrackIndex !== null &&
    indexes.includes(state.currentTrackIndex);
  let index: number;
  if (state.is_shuffle && direction === 1) {
    const available =
      currentInScope && indexes.length > 1
        ? indexes.filter((index) => index !== state.currentTrackIndex)
        : indexes;
    index = available[Math.floor(Math.random() * available.length)];
  } else {
    const position =
      state.currentTrackIndex === null
        ? -1
        : indexes.indexOf(state.currentTrackIndex);
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
    return { currentTrackIndex: index, currentTrack: track };
  }
  if (direction === 1 && currentInScope && indexes.length === 1) return {};

  const history = state.playHistory
    .slice(
      0,
      direction === 1 ? Math.max(state.historyPosition + 1, 0) : undefined,
    )
    .filter((track) => scopedIndexById.has(track.id));
  if (history.length === 0 && currentInScope && state.currentTrack) {
    history.push(state.currentTrack);
  }
  const playHistory =
    direction === 1 ? [...history, track] : [track, ...history];
  return {
    currentTrackIndex: index,
    currentTrack: track,
    playHistory,
    historyPosition: direction === 1 ? playHistory.length - 1 : 0,
  };
}
