import { describe, it, expect, vi, beforeEach } from "vitest";
import { get } from "svelte/store";
import { authConfigStore } from "./authConfigStore";
import * as apiClient from "$lib/services/apiClient";

// Mock apiClient
vi.mock("$lib/services/apiClient", () => ({
  fetchMagicLinkUrl: vi.fn(),
}));

describe("authConfigStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should initialize with default values", () => {
    const state = get(authConfigStore);
    expect(state.isAuthEnabled).toBe(false);
    expect(state.magicLinkUrl).toBe("");
  });

  it("should set auth enabled when magic link URL is returned", async () => {
    const mockUrl = "/login?token=test-secret-key";
    vi.mocked(apiClient.fetchMagicLinkUrl).mockResolvedValue(mockUrl);

    await authConfigStore.initialize();

    const state = get(authConfigStore);
    expect(state.isAuthEnabled).toBe(true);
    expect(state.magicLinkUrl).toBe(mockUrl);
  });

  it("should set auth disabled when empty URL is returned", async () => {
    vi.mocked(apiClient.fetchMagicLinkUrl).mockResolvedValue("");

    await authConfigStore.initialize();

    const state = get(authConfigStore);
    expect(state.isAuthEnabled).toBe(false);
    expect(state.magicLinkUrl).toBe("");
  });

  it("should handle initialization errors gracefully", async () => {
    vi.mocked(apiClient.fetchMagicLinkUrl).mockRejectedValue(
      new Error("Network error"),
    );
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    await authConfigStore.initialize();

    const state = get(authConfigStore);
    expect(state.isAuthEnabled).toBe(false);
    expect(state.magicLinkUrl).toBe("");
    expect(consoleSpy).toHaveBeenCalledWith(
      "Failed to initialize auth config:",
      expect.any(Error),
    );

    consoleSpy.mockRestore();
  });
});
