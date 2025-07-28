import { writable } from "svelte/store";

export interface PermissionsState {
  can_write_music_files: boolean;
}

const initialState: PermissionsState = {
  can_write_music_files: false,
};

export const permissionsStore = writable<PermissionsState>(initialState);
