import type { Track } from "$lib/types";

function markerText(value: string): string {
  return value
    .toLowerCase()
    .replace(/[()[\]\-_.]/g, " ")
    .split(/\s+/)
    .filter(Boolean)
    .join(" ");
}

export function hasRightVersionMarker(track: Track): boolean {
  const text = markerText(`${track.title} ${track.filename}`);
  return text.includes("right version") || text.includes("right ver");
}

export function hasAiCoverMarker(track: Track): boolean {
  return markerText(`${track.title} ${track.filename}`).includes("ai cover");
}
