export function formatArtistsForDisplay(artistString: string): string {
  return artistString
    .split(";")
    .map((artist) => artist.trim())
    .filter(Boolean)
    .join(", ");
}

export function formatDuration(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.floor(seconds % 60);
  return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
}
