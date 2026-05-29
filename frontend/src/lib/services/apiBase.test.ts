import { afterEach, describe, expect, it, vi } from "vitest";

async function loadApiBase() {
  vi.resetModules();
  return await import("./apiBase");
}

describe("apiBase", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it("uses the public host outside SSR", async () => {
    vi.stubEnv("SSR", false);
    vi.stubEnv("VITE_INTERNAL_API_HOST", "http://backend:8001");
    vi.stubEnv("VITE_PUBLIC_API_HOST", "https://music.example");

    const apiBase = await loadApiBase();

    expect(apiBase.API_BASE_URL).toBe("https://music.example/api/v1");
    expect(apiBase.PUBLIC_API_HOST).toBe("https://music.example");
  });

  it("uses the internal host only during SSR", async () => {
    vi.stubEnv("SSR", true);
    vi.stubEnv("VITE_INTERNAL_API_HOST", "http://backend:8001");
    vi.stubEnv("VITE_PUBLIC_API_HOST", "https://music.example");

    const apiBase = await loadApiBase();

    expect(apiBase.API_BASE_URL).toBe("http://backend:8001/api/v1");
    expect(apiBase.PUBLIC_API_HOST).toBe("https://music.example");
  });
});
