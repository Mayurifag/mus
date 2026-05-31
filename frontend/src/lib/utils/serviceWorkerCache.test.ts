import { describe, expect, it } from "vitest";
import { isStaticAsset, shouldSkipCache } from "./serviceWorkerCache";

describe("serviceWorkerCache", () => {
  it("does not cache API or HLS media routes", () => {
    expect(shouldSkipCache("/api/v1/tracks")).toBe(true);
    expect(shouldSkipCache("/api/v1/tracks/1/hls/2001/index.m3u8")).toBe(true);
    expect(shouldSkipCache("/api/v1/tracks/1/hls/2001/segment-00000.m4s")).toBe(
      true,
    );
  });

  it("only treats immutable app assets as static cache entries", () => {
    expect(isStaticAsset("/_app/immutable/chunks/app.js")).toBe(true);
    expect(isStaticAsset("/images/no-cover.svg")).toBe(false);
  });
});
