import { describe, expect, it } from "vitest";
import { formatCutTime, isValidCut, parseCutTime } from "./trackCut";

describe("audio cut times", () => {
  it("accepts seconds and minute timestamps with millisecond precision", () => {
    expect(parseCutTime(" 90.125 ")).toBe(90.125);
    expect(parseCutTime("1:30.125")).toBe(90.125);
    expect(parseCutTime("123:59.999")).toBe(7439.999);
    expect(formatCutTime(90.125)).toBe("1:30.125");
    expect(formatCutTime(59.9999)).toBe("1:00.000");
  });

  it.each([
    "",
    "-1",
    "1:60",
    "1:2",
    "1:00:30",
    "NaN",
    "Infinity",
    "12oops",
    "1.1234",
  ])("rejects invalid timestamp %j", (value) => {
    expect(parseCutTime(value)).toBeNaN();
  });

  it("allows intro, middle, and outro removal but not the whole track", () => {
    expect(isValidCut({ start_seconds: 0, end_seconds: 10 }, 60)).toBe(true);
    expect(isValidCut({ start_seconds: 10.125, end_seconds: 30.5 }, 60)).toBe(
      true,
    );
    expect(isValidCut({ start_seconds: 50, end_seconds: 60 }, 60)).toBe(true);
    for (const [start_seconds, end_seconds] of [
      [0, 60],
      [1, 1],
      [20, 10],
      [-1, 10],
      [10, 61],
      [NaN, 10],
      [10, Infinity],
    ]) {
      expect(isValidCut({ start_seconds, end_seconds }, 60)).toBe(false);
    }
  });
});
