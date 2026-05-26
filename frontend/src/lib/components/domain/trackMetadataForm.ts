import type { Track } from "$lib/types";

export type TrackMetadataMode = "edit" | "create";

export type ArtistRow = {
  id: number;
  value: string;
};

export type TrackUpdatePayload = {
  title?: string;
  artist?: string;
  rename_file?: boolean;
  artwork_url?: string;
};

export function sanitizeMetadataInput(value: string): string {
  return value.replace(/[<>:"/\\|?*]/g, "");
}

export function sanitizeArtistRows(artists: ArtistRow[]): ArtistRow[] {
  return artists.map((artist) => ({
    ...artist,
    value: sanitizeMetadataInput(artist.value),
  }));
}

export function formatArtistString(artists: ArtistRow[]): string {
  return artists
    .map((artist) => artist.value.trim())
    .filter(Boolean)
    .join("; ");
}

export function previewFilename(
  artists: ArtistRow[],
  title: string,
  fallbackFilename = "untitled.mp3",
): string {
  const artistNames = artists
    .filter((artist) => artist.value.trim())
    .map((artist) => artist.value.trim())
    .join(", ");
  const trimmedTitle = title.trim();

  if (!artistNames && !trimmedTitle) return fallbackFilename;
  if (!artistNames) return `${trimmedTitle}.mp3`;
  if (!trimmedTitle) return `${artistNames}.mp3`;

  return `${artistNames} - ${trimmedTitle}.mp3`;
}

export function displayedFilename(
  mode: TrackMetadataMode,
  renameFile: boolean,
  generatedFilename: string,
  track?: Track,
): string {
  if (mode === "create") return generatedFilename;
  return renameFile ? generatedFilename : track?.filename || "";
}

export function isTrackMetadataFormValid({
  mode,
  title,
  primaryArtist,
  renameFile,
  filenameTooLong,
}: {
  mode: TrackMetadataMode;
  title: string;
  primaryArtist?: string;
  renameFile: boolean;
  filenameTooLong: boolean;
}): boolean {
  const filenameValid =
    mode === "edit" ? !renameFile || !filenameTooLong : !filenameTooLong;
  return title.trim().length > 0 && !!primaryArtist?.trim() && filenameValid;
}

export function buildTrackUpdatePayload({
  track,
  title,
  artist,
  renameFile,
  filename,
  artworkUrl,
}: {
  track?: Track;
  title: string;
  artist: string;
  renameFile: boolean;
  filename: string;
  artworkUrl?: string;
}): TrackUpdatePayload {
  const payload: TrackUpdatePayload = {};
  if (!track) return payload;
  if (title !== track.title) payload.title = title;
  if (artist !== track.artist) payload.artist = artist;
  if (renameFile && filename !== track.filename) payload.rename_file = true;
  if (artworkUrl) payload.artwork_url = artworkUrl;
  return payload;
}
