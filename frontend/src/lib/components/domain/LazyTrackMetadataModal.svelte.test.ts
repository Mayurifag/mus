import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import "@testing-library/jest-dom/vitest";
import LazyTrackMetadataModal from "./LazyTrackMetadataModal.svelte";

const loading = vi.hoisted(() => {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
});

vi.mock("./TrackMetadataModal.svelte", async () => {
  await loading.promise;
  throw new Error("Failed to load editor");
});

describe("lazy track editor", () => {
  it("closes on Escape while the editor is loading", async () => {
    const onClose = vi.fn();
    render(LazyTrackMetadataModal, {
      props: { open: true, mode: "edit", onClose },
    });

    expect(
      screen.getByRole("dialog", { name: "Track editor" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Loading track editor",
    );
    await fireEvent.keyDown(document, { key: "Escape" });
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    loading.resolve();
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
  });

  it("lets the user close an editor that failed to load", async () => {
    const onClose = vi.fn();
    render(LazyTrackMetadataModal, {
      props: { open: true, mode: "edit", onClose },
    });

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not load track editor",
    );
    await fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalledOnce();
  });
});
