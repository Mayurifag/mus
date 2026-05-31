import { describe, expect, it } from "vitest";
import {
  audioStreamRangeHeader,
  boundedAudioStreamRangeHeader,
} from "./audioStreamRange";

describe("audioStreamRange", () => {
  it("builds bounded audio stream ranges", () => {
    expect(audioStreamRangeHeader(0)).toBe("bytes=0-262143");
    expect(audioStreamRangeHeader(3)).toBe("bytes=3-262146");
  });

  it("bounds missing and open-ended audio stream ranges", () => {
    expect(boundedAudioStreamRangeHeader(null)).toBe("bytes=0-262143");
    expect(boundedAudioStreamRangeHeader("bytes=3-")).toBe("bytes=3-262146");
    expect(boundedAudioStreamRangeHeader("bytes=3-262146")).toBeNull();
    expect(boundedAudioStreamRangeHeader("bytes=3-999999")).toBe(
      "bytes=3-262146",
    );
    expect(boundedAudioStreamRangeHeader("bytes=3-9")).toBeNull();
    expect(boundedAudioStreamRangeHeader("bytes=-1024")).toBeNull();
  });
});
