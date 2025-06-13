import { writable } from "svelte/store";

interface AuthState {
  authEnabled: boolean;
  isAuthenticated: boolean;
}

export const authStore = writable<AuthState>({
  authEnabled: false,
  isAuthenticated: false,
});

export function initializeAuthStore(initialData: AuthState): void {
  authStore.set(initialData);
}
