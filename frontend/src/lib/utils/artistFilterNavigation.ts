import { browser } from "$app/environment";
import { tick } from "svelte";
import { get } from "svelte/store";
import { trackStore } from "$lib/stores/trackStore";

let returnScrollY: number | null = null;

export function selectArtistFilter(artist: string) {
  if (browser && get(trackStore).selectedArtist === null) {
    returnScrollY = window.scrollY;
  }

  trackStore.setArtistFilter(artist);
}

export async function clearArtistFilter() {
  const scrollY = returnScrollY;

  trackStore.clearArtistFilter();
  returnScrollY = null;

  if (!browser || scrollY === null) return;

  await tick();
  window.scrollTo({ top: scrollY, left: window.scrollX, behavior: "auto" });
}
