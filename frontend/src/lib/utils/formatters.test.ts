import { describe, expect, it } from "vitest";
import {
  formatArtistsForDisplay,
  normalizeArtistsForStorage,
  parseArtists,
} from "./formatters";

describe("formatters", () => {
  it("parses semicolon and comma separated artists", () => {
    expect(parseArtists("aikko,katanacss; INSPACE, playingtheangel")).toEqual([
      "aikko",
      "katanacss",
      "INSPACE",
      "playingtheangel",
    ]);
  });

  it("formats artists with readable comma spacing", () => {
    expect(formatArtistsForDisplay("aikko,katanacss,INSPACE")).toBe(
      "aikko, katanacss, INSPACE",
    );
  });

  it("normalizes artists for storage", () => {
    expect(normalizeArtistsForStorage("aikko,katanacss,INSPACE")).toBe(
      "aikko; katanacss; INSPACE",
    );
  });
});
