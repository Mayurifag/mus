import { describe, it, expect } from "vitest";
import { render } from "@testing-library/svelte";
import Slider from "./slider.svelte";
import type { TimeRange } from "$lib/types";

describe("Slider component", () => {
  it("renders without bufferedRanges", () => {
    const { container } = render(Slider, {
      value: 50,
      max: 100,
    });

    const slider = container.querySelector('[role="slider"]');
    expect(slider).toBeInTheDocument();
  });

  it("renders without bufferedRanges when undefined", () => {
    const { container } = render(Slider, {
      value: 50,
      max: 100,
      bufferedRanges: undefined,
    });

    const slider = container.querySelector('[role="slider"]');
    expect(slider).toBeInTheDocument();

    const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
    expect(bufferedSegments).toHaveLength(0);
  });

  it("renders without bufferedRanges when empty array", () => {
    const { container } = render(Slider, {
      value: 50,
      max: 100,
      bufferedRanges: [],
    });

    const slider = container.querySelector('[role="slider"]');
    expect(slider).toBeInTheDocument();

    const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
    expect(bufferedSegments).toHaveLength(0);
  });

  it("renders single buffered range with correct positioning", () => {
    const bufferedRanges: TimeRange[] = [{ start: 10, end: 50 }];

    const { container } = render(Slider, {
      value: 25,
      max: 100,
      bufferedRanges,
    });

    const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
    expect(bufferedSegments).toHaveLength(1);

    const segment = bufferedSegments[0] as HTMLElement;
    expect(segment.style.left).toBe("10%");
    expect(segment.style.width).toBe("40%");
  });

  it("renders multiple buffered ranges with correct positioning", () => {
    const bufferedRanges: TimeRange[] = [
      { start: 0, end: 30 },
      { start: 60, end: 90 },
    ];

    const { container } = render(Slider, {
      value: 45,
      max: 100,
      bufferedRanges,
    });

    const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
    expect(bufferedSegments).toHaveLength(2);

    const firstSegment = bufferedSegments[0] as HTMLElement;
    expect(firstSegment.style.left).toBe("0%");
    expect(firstSegment.style.width).toBe("30%");

    const secondSegment = bufferedSegments[1] as HTMLElement;
    expect(secondSegment.style.left).toBe("60%");
    expect(secondSegment.style.width).toBe("30%");
  });

  it("calculates correct percentages for different max values", () => {
    const bufferedRanges: TimeRange[] = [{ start: 30, end: 90 }];

    const { container } = render(Slider, {
      value: 60,
      max: 180,
      bufferedRanges,
    });

    const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
    expect(bufferedSegments).toHaveLength(1);

    const segment = bufferedSegments[0] as HTMLElement;
    expect(segment.style.left).toMatch(/^16\.666666666666\d+%$/);
    expect(segment.style.width).toMatch(/^33\.333333333333\d+%$/);
  });

  it("applies correct CSS classes to buffered segments", () => {
    const bufferedRanges: TimeRange[] = [{ start: 10, end: 50 }];

    const { container } = render(Slider, {
      value: 25,
      max: 100,
      bufferedRanges,
    });

    const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
    expect(bufferedSegments).toHaveLength(1);

    const segment = bufferedSegments[0] as HTMLElement;
    expect(segment).toHaveClass("bg-accent/20");
    expect(segment).toHaveClass("absolute");
    expect(segment).toHaveClass("top-0");
    expect(segment).toHaveClass("h-full");
  });

  it("ensures buffered segments are layered behind progress range", () => {
    const bufferedRanges: TimeRange[] = [{ start: 10, end: 50 }];

    const { container } = render(Slider, {
      value: 25,
      max: 100,
      bufferedRanges,
    });

    const track = container.querySelector(".bg-muted");
    expect(track).toBeInTheDocument();

    const children = Array.from(track?.children || []);
    const bufferedSegmentIndex = children.findIndex((child) =>
      child.classList.contains("bg-accent/20"),
    );
    const progressRangeIndex = children.findIndex(
      (child) =>
        child.classList.contains("bg-accent") &&
        !child.classList.contains("bg-accent/20"),
    );

    expect(bufferedSegmentIndex).toBeLessThan(progressRangeIndex);
  });

  it("handles edge case with zero-width range", () => {
    const bufferedRanges: TimeRange[] = [{ start: 50, end: 50 }];

    const { container } = render(Slider, {
      value: 25,
      max: 100,
      bufferedRanges,
    });

    const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
    expect(bufferedSegments).toHaveLength(1);

    const segment = bufferedSegments[0] as HTMLElement;
    expect(segment.style.left).toBe("50%");
    expect(segment.style.width).toBe("0%");
  });

  it("handles ranges that exceed max value", () => {
    const bufferedRanges: TimeRange[] = [{ start: 80, end: 120 }];

    const { container } = render(Slider, {
      value: 50,
      max: 100,
      bufferedRanges,
    });

    const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
    expect(bufferedSegments).toHaveLength(1);

    const segment = bufferedSegments[0] as HTMLElement;
    expect(segment.style.left).toBe("80%");
    expect(segment.style.width).toBe("40%");
  });
});
