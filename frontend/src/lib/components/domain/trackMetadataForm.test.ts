import { describe, expect, it } from "vitest";
import { previewFilename } from "./trackMetadataForm";

describe("filename preview", () => {
  it("uses the backend's NFC and semicolon-separated artist convention", () => {
    expect(
      previewFilename(
        [{ id: 0, value: "Cafe\u0301; Guest/Name" }],
        "Re\u0301sume\u0301?",
        "original.wav",
      ),
    ).toBe("Café, GuestName - Résumé.wav");
  });
});
