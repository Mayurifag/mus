import { vi, describe, it, expect, beforeEach } from "vitest";
import { load } from "../+page";

describe("auth callback page load function", () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it("should return token when token is present in URL", async () => {
    const mockUrl = new URL(
      "http://localhost:5173/auth/callback?token=test-secret-key",
    );
    const mockEvent = {
      url: mockUrl,
    } as Parameters<typeof load>[0];

    const result = await load(mockEvent);

    expect(result).toEqual({ token: "test-secret-key" });
  });

  it("should return null token when token is not present", async () => {
    const mockUrl = new URL("http://localhost:5173/auth/callback");
    const mockEvent = {
      url: mockUrl,
    } as Parameters<typeof load>[0];

    const result = await load(mockEvent);

    expect(result).toEqual({ token: null });
  });

  it("should return empty string token when token parameter is empty", async () => {
    const mockUrl = new URL("http://localhost:5173/auth/callback?token=");
    const mockEvent = {
      url: mockUrl,
    } as Parameters<typeof load>[0];

    const result = await load(mockEvent);

    expect(result).toEqual({ token: "" });
  });
});
