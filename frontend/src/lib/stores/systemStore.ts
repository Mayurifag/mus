import { writable } from "svelte/store";

export const ytDlpVersionStore = writable<string | null>(null);
