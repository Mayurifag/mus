import { describe, expect, it } from "vitest";
import {
  isAudioStream,
  isStaticAsset,
  shouldSkipCache,
} from "./serviceWorkerCache";

describe("serviceWorkerCache", () => {
  it("does not cache API or media stream routes", () => {
    expect(shouldSkipCache("/api/v1/tracks")).toBe(true);
    expect(shouldSkipCache("/api/v1/tracks/1/stream")).toBe(true);
  });

  it("only treats immutable app assets as static cache entries", () => {
    expect(isStaticAsset("/_app/immutable/chunks/app.js")).toBe(true);
    expect(isStaticAsset("/images/no-cover.svg")).toBe(false);
  });

  it("detects audio stream routes", () => {
    expect(isAudioStream("/api/v1/tracks/1/stream")).toBe(true);
    expect(isAudioStream("/api/v1/tracks/1/covers/small.webp")).toBe(false);
    expect(isAudioStream("/api/v1/tracks/not-a-number/stream")).toBe(false);
  });
});
