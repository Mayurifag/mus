import { writable } from "svelte/store";
import { fetchMagicLinkUrl } from "$lib/services/apiClient";

interface AuthConfig {
  isAuthEnabled: boolean;
  magicLinkUrl: string;
}

const createAuthConfigStore = () => {
  const { subscribe, set } = writable<AuthConfig>({
    isAuthEnabled: false,
    magicLinkUrl: "",
  });

  return {
    subscribe,
    async initialize() {
      try {
        const url = await fetchMagicLinkUrl();
        set({
          isAuthEnabled: url !== "",
          magicLinkUrl: url,
        });
      } catch (error) {
        console.error("Failed to initialize auth config:", error);
        set({
          isAuthEnabled: false,
          magicLinkUrl: "",
        });
      }
    },
  };
};

export const authConfigStore = createAuthConfigStore();
