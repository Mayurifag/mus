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
        const relativeUrl = await fetchMagicLinkUrl();
        const fullUrl = relativeUrl
          ? `${window.location.origin}${relativeUrl}`
          : "";
        set({
          isAuthEnabled: relativeUrl !== "",
          magicLinkUrl: fullUrl,
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
