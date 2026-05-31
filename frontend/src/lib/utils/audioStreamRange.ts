const AUDIO_STREAM_CHUNK_BYTES = 256 * 1024;

export function audioStreamRangeHeader(start: number): string {
  return `bytes=${start}-${start + AUDIO_STREAM_CHUNK_BYTES - 1}`;
}

export function boundedAudioStreamRangeHeader(
  rangeHeader: string | null,
): string | null {
  if (rangeHeader === null) return audioStreamRangeHeader(0);

  const match = rangeHeader.trim().match(/^bytes=(\d+)-(\d*)$/);
  if (!match) return null;

  const start = Number(match[1]);
  if (!Number.isSafeInteger(start)) return null;

  const end = match[2] === "" ? null : Number(match[2]);
  if (end !== null && !Number.isSafeInteger(end)) return null;
  if (end !== null && end < start + AUDIO_STREAM_CHUNK_BYTES) return null;

  return audioStreamRangeHeader(start);
}
