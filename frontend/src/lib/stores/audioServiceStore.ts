import { writable } from "svelte/store";
import type { AudioService } from "$lib/services/AudioService";

export const audioServiceStore = writable<AudioService | undefined>(undefined);
