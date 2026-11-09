import type { AudioCut, Track } from "$lib/types";
import { parseArtists } from "$lib/utils/formatters";

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
  tags?: string[];
  remove_section?: AudioCut;
};

function oneEditApart(left: string, right: string): boolean {
  const a = Array.from(left);
  const b = Array.from(right);
  if (Math.abs(a.length - b.length) > 1) return false;
  let i = 0;
  let j = 0;
  let edits = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      i++;
      j++;
      continue;
    }
    if (++edits > 1) return false;
    if (a.length >= b.length) i++;
    if (a.length <= b.length) j++;
  }
  return edits + Number(i < a.length || j < b.length) <= 1;
}

export function artistSuggestions(
  value: string,
  counts: Record<string, number>,
): string[] {
  const key = (name: string) => name.normalize("NFC").trim().toLowerCase();
  const compact = (name: string) => key(name).replace(/[^\p{L}\p{N}]/gu, "");
  const query = key(value);
  const compactQuery = compact(value);
  if (!query || Array.from(query).length > 80) return [];
  return Object.keys(counts)
    .filter((name) => name !== value && Array.from(name).length <= 80)
    .map((name) => {
      const candidate = key(name);
      const compactCandidate = compact(name);
      const rank =
        candidate === query
          ? 0
          : compactQuery && compactCandidate === compactQuery
            ? 1
            : candidate.includes(query) || query.includes(candidate)
              ? 2
              : query.length >= 4 && oneEditApart(query, candidate)
                ? 3
                : 4;
      return { name, rank };
    })
    .filter(({ rank }) => rank < 4)
    .sort(
      (a, b) =>
        a.rank - b.rank ||
        counts[b.name] - counts[a.name] ||
        a.name.localeCompare(b.name),
    )
    .slice(0, 5)
    .map(({ name }) => name);
}

export function existingTitleSuggestions(
  title: string,
  artists: ArtistRow[],
  duration: number | null | undefined,
  tracks: Track[],
): string[] {
  if (!duration || duration <= 0 || !Number.isFinite(duration)) return [];
  const key = (value: string) => value.normalize("NFC").trim().toLowerCase();
  const expectedArtists = artists
    .map((artist) => key(artist.value))
    .filter(Boolean)
    .sort();
  if (!key(title) || expectedArtists.length === 0) return [];
  return [
    ...new Set(
      tracks
        .filter(
          (track) =>
            track.title !== title &&
            key(track.title) === key(title) &&
            track.duration > 0 &&
            Math.abs(track.duration - duration) <= 3 &&
            JSON.stringify(parseArtists(track.artist).map(key).sort()) ===
              JSON.stringify(expectedArtists),
        )
        .map((track) => track.title),
    ),
  ];
}

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
