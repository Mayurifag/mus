import type { TrackState } from "$lib/stores/trackStore";
import { trackIndex } from "$lib/stores/trackNavigation";
import { parseArtists } from "$lib/utils/formatters";
import { trackCategoryNames } from "$lib/utils/categoryMarkers";

type LibraryFilter = Pick<TrackState, "selectedArtist" | "selectedCategory">;
const selectedByLibrary = new WeakMap<
  TrackState["tracks"],
  LibraryFilter & { indexes: readonly number[] }
>();

export function selectedTrackIndexes({
  tracks,
  selectedArtist,
  selectedCategory,
}: LibraryFilter & Pick<TrackState, "tracks">): readonly number[] {
  const selected = selectedByLibrary.get(tracks);
  if (
    selected &&
    selected.selectedArtist === selectedArtist &&
    selected.selectedCategory === selectedCategory
  ) {
    return selected.indexes;
  }

  const indexes = tracks.reduce<number[]>((result, track, index) => {
    if (
      (!selectedArtist ||
        parseArtists(track.artist).includes(selectedArtist)) &&
      (!selectedCategory ||
        trackCategoryNames(track).includes(selectedCategory))
    ) {
      result.push(index);
    }
    return result;
  }, []);
  selectedByLibrary.set(tracks, { selectedArtist, selectedCategory, indexes });
  return indexes;
}

export function navigateFilteredTracks(
  state: TrackState,
  indexes: readonly number[],
  direction: 1 | -1,
): Partial<TrackState> {
  const currentTrackIndex = trackIndex(state.tracks, state.currentTrackId);
  const position = indexes.indexOf(currentTrackIndex);
  const inScope = state.is_shuffle ? new Set(indexes) : null;
  if (inScope) {
    for (
      let cursor = state.historyPosition + direction;
      cursor >= 0 && cursor < state.playHistoryIds.length;
      cursor += direction
    ) {
      const id = state.playHistoryIds[cursor];
      if (inScope.has(trackIndex(state.tracks, id))) {
        return { currentTrackId: id, historyPosition: cursor };
      }
    }
  }

  const currentInScope = position !== -1;
  let index: number;
  if (inScope && direction === 1) {
    if (currentInScope && indexes.length === 1) return {};
    const offset = Math.floor(
      Math.random() * (indexes.length - Number(currentInScope)),
    );
    index = indexes[offset + Number(currentInScope && offset >= position)];
  } else {
    const nextPosition =
      position === -1
        ? direction === 1
          ? 0
          : indexes.length - 1
        : (position + direction + indexes.length) % indexes.length;
    index = indexes[nextPosition];
  }
  const track = state.tracks[index];
  if (!inScope) return { currentTrackId: track.id };

  const history = state.playHistoryIds
    .slice(
      0,
      direction === 1 ? Math.max(state.historyPosition + 1, 0) : undefined,
    )
    .filter((id) => inScope.has(trackIndex(state.tracks, id)));
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
