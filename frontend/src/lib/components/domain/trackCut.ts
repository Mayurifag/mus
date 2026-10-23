import type { AudioCut } from "$lib/types";

export function parseCutTime(value: string): number {
  const text = value.trim();
  if (!/^\d+(?::[0-5]\d)?(?:\.\d{1,3})?$/.test(text)) return NaN;
  const parts = text.split(":").map(Number);
  return parts.length === 2 ? parts[0] * 60 + parts[1] : parts[0];
}

export function formatCutTime(seconds: number): string {
  const milliseconds = Math.round(seconds * 1000);
  const minutes = Math.floor(milliseconds / 60000);
  const remainder = ((milliseconds % 60000) / 1000).toFixed(3).padStart(6, "0");
  return `${minutes}:${remainder}`;
}

export function isValidCut(cut: AudioCut, duration: number): boolean {
  return (
    Number.isFinite(cut.start_seconds) &&
    Number.isFinite(cut.end_seconds) &&
    cut.start_seconds >= 0 &&
    cut.end_seconds > cut.start_seconds &&
    cut.end_seconds <= duration + 0.001 &&
    duration - (cut.end_seconds - cut.start_seconds) >= 0.1
  );
}
