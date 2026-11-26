import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { tick } from "svelte";
import { rgbaToThumbHash } from "thumbhash";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import TrackCover from "./TrackCover.svelte";

const url = "/covers/test.webp";
const thumbhash = btoa(
  String.fromCharCode(...rgbaToThumbHash(1, 1, [123, 45, 67, 255])),
);
let notify: IntersectionObserverCallback;

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

// The cover component shares its observer across rows.
it("defers far covers and prioritizes covers above the fixed footer", async () => {
  vi.stubGlobal(
    "IntersectionObserver",
    class {
      observe = vi.fn();
      unobserve = vi.fn();
      constructor(callback: IntersectionObserverCallback) {
        notify = callback;
      }
    },
  );
  const footer = document.createElement("div");
  footer.dataset.playerFooter = "";
  footer.getBoundingClientRect = () => new DOMRect(0, 650, 390, 190);
  document.body.append(footer);
  try {
    for (const title of ["Far", "Buffered", "Visible"]) {
      render(TrackCover, { url, title });
    }
    const [far, buffered, visible] = ["Far", "Buffered", "Visible"].map(
      (title) => screen.getByAltText(`Album art for ${title}`),
    );
    function entry(
      img: HTMLElement,
      top: number,
      isIntersecting: boolean,
    ): IntersectionObserverEntry {
      return {
        target: img.parentElement!,
        isIntersecting,
        boundingClientRect: new DOMRect(0, top, 56, 56),
        intersectionRect: new DOMRect(),
        intersectionRatio: 0,
        rootBounds: null,
        time: 0,
      };
    }
    notify(
      [
        entry(far, 1200, false),
        entry(buffered, 700, true),
        entry(visible, 400, true),
      ],
      { unobserve: vi.fn() } as unknown as IntersectionObserver,
    );
    await tick();
    expect(far).not.toHaveAttribute("src");
    expect(buffered).toHaveAttribute("src", url);
    expect(buffered).toHaveAttribute("fetchpriority", "low");
    expect(visible).toHaveAttribute("src", url);
    expect(visible).toHaveAttribute("fetchpriority", "high");
  } finally {
    footer.remove();
  }
});

it("replaces a slow cover's preview with the loaded image", async () => {
  render(TrackCover, { url, thumbhash, title: "Slow" });
  const img = screen.getByAltText("Album art for Slow");
  expect(img).toHaveClass("opacity-0");
  expect(img.parentElement?.style.backgroundImage).toBe("");
  await vi.advanceTimersByTimeAsync(100);
  expect(img.parentElement?.style.backgroundImage).toContain(
    "data:image/png;base64,",
  );
  await fireEvent.load(img);
  expect(img.parentElement?.style.backgroundImage).toBe("");
  expect(img).not.toHaveClass("opacity-0");
});

it("does not generate a preview for an image already present in the browser cache", async () => {
  render(TrackCover, { url, thumbhash, title: "Cached" });
  const img = screen.getByAltText("Album art for Cached");
  Object.defineProperties(img, {
    complete: { value: true },
    naturalWidth: { value: 80 },
  });
  await vi.advanceTimersByTimeAsync(100);
  expect(img.parentElement?.style.backgroundImage).toBe("");
});

it("clears the preview and uses the no-cover icon on error", async () => {
  render(TrackCover, { url, thumbhash, title: "Broken" });
  const img = screen.getByAltText("Album art for Broken");
  await vi.advanceTimersByTimeAsync(100);
  await fireEvent.error(img);
  expect(screen.getByAltText("No Album Cover")).toHaveAttribute(
    "src",
    "/images/no-cover.svg",
  );
  expect(img.parentElement?.style.backgroundImage).toBe("");
});
