import { effectStats } from "$lib/stores/effectMonitorStore";

export function updateEffectStats(name: string) {
  effectStats.update((stats) => {
    const current = stats.get(name) || { count: 0, lastTriggered: 0 };
    stats.set(name, {
      count: current.count + 1,
      lastTriggered: Date.now(),
    });
    return stats;
  });
}
