import { writable } from "svelte/store";

export interface EffectStat {
  count: number;
  lastTriggered: number;
}

export const effectStats = writable<Map<string, EffectStat>>(new Map());

export function clearEffectStats(): void {
  effectStats.set(new Map());
}
