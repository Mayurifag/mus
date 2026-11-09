import { describe, expect, it } from "vitest";
import {
  formatArtistsForDisplay,
  normalizeArtistsForStorage,
  parseArtists,
} from "./formatters";

describe("formatters", () => {
  it("preserves commas inside artist names", () => {
    expect(parseArtists("Tyler, The Creator; Guest")).toEqual([
      "Tyler, The Creator",
      "Guest",
    ]);
    expect(normalizeArtistsForStorage("Tyler, The Creator; Guest")).toBe(
      "Tyler, The Creator; Guest",
    );
  });

  it("parses semicolon separated artists and preserves commas", () => {
    expect(parseArtists("aikko,katanacss; INSPACE, playingtheangel")).toEqual([
      "aikko,katanacss",
      "INSPACE, playingtheangel",
    ]);
  });

  it("formats artists with readable comma spacing", () => {
    expect(formatArtistsForDisplay("aikko; katanacss; INSPACE")).toBe(
      "aikko, katanacss, INSPACE",
    );
  });

  it("normalizes artists for storage", () => {
    expect(normalizeArtistsForStorage("aikko; katanacss;INSPACE")).toBe(
      "aikko; katanacss; INSPACE",
    );
  });
});
