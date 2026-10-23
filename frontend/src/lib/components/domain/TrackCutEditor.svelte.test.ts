import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";
import "@testing-library/jest-dom/vitest";
import TrackCutEditor from "./TrackCutEditor.svelte";
import {
  fetchTrackCutInfo,
  fetchTrackCutPreview,
} from "$lib/services/apiClient";
import type { Track } from "$lib/types";

vi.mock("$lib/services/apiClient", () => ({
  fetchTrackCutInfo: vi.fn(),
  fetchTrackCutPreview: vi.fn(),
}));

const track: Track = {
  id: "track-id",
  title: "Song",
  artist: "Artist",
  duration: 60,
  filename: "song.mp3",
  has_cover: false,
  updated_at: 1,
  cover_small_url: null,
  cover_original_url: null,
  hls_url: "/song.m3u8",
};

describe("TrackCutEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(fetchTrackCutInfo).mockResolvedValue({ duration_seconds: 60 });
    vi.mocked(fetchTrackCutPreview).mockResolvedValue(
      new Blob(["audio"], { type: "audio/wav" }),
    );
    vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(() => {});
    vi.spyOn(HTMLMediaElement.prototype, "play").mockResolvedValue();
    vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:preview");
    vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
  });

  it("loads precise duration only when expanded and exposes keyboard-accessible handles", async () => {
    render(TrackCutEditor, { track, onChange: vi.fn() });
    expect(fetchTrackCutInfo).not.toHaveBeenCalled();
    await fireEvent.click(
      screen.getByRole("button", { name: "Select section" }),
    );
    expect(await screen.findByLabelText("Remove from")).toHaveValue("0:25.000");
    expect(
      screen.getByRole("slider", { name: "Cut start" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "Cut end" })).toBeInTheDocument();
    expect(fetchTrackCutPreview).not.toHaveBeenCalled();
  });

  it("previews the selected join and clears stale audio and confirmation when the cut changes", async () => {
    const { unmount } = render(TrackCutEditor, { track, onChange: vi.fn() });
    await fireEvent.click(
      screen.getByRole("button", { name: "Select section" }),
    );
    await screen.findByLabelText("Remove from");
    await fireEvent.click(screen.getByRole("button", { name: "Preview join" }));
    const preview = await screen.findByLabelText("Cut join preview");
    expect(preview).toHaveAttribute("src", "blob:preview");
    expect(fetchTrackCutPreview).toHaveBeenCalledWith(
      "track-id",
      { start_seconds: 25, end_seconds: 35 },
      false,
      expect.any(AbortSignal),
    );
    await fireEvent.click(
      screen.getByRole("checkbox", { name: "Apply this cut when saving" }),
    );
    await fireEvent.input(screen.getByLabelText("Remove from"), {
      target: { value: "0:24.500" },
    });
    expect(screen.queryByLabelText("Cut join preview")).not.toBeInTheDocument();
    expect(screen.getByRole("checkbox")).not.toBeChecked();
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:preview");
    await fireEvent.click(
      screen.getByRole("button", { name: "Hear removed section" }),
    );
    await screen.findByLabelText("Selected section preview");
    expect(fetchTrackCutPreview).toHaveBeenLastCalledWith(
      "track-id",
      { start_seconds: 24.5, end_seconds: 35 },
      true,
      expect.any(AbortSignal),
    );
    unmount();
    expect(URL.revokeObjectURL).toHaveBeenCalledTimes(2);
  });

  it("rejects empty, reversed, and whole-track selections", async () => {
    render(TrackCutEditor, { track, onChange: vi.fn() });
    await fireEvent.click(
      screen.getByRole("button", { name: "Select section" }),
    );
    const start = await screen.findByLabelText("Remove from");
    const end = screen.getByLabelText("To");
    for (const [from, to] of [
      ["", "30"],
      ["40", "30"],
      ["0", "60"],
    ]) {
      await fireEvent.input(start, { target: { value: from } });
      await fireEvent.input(end, { target: { value: to } });
      expect(
        screen.getByRole("button", { name: "Preview join" }),
      ).toBeDisabled();
      expect(screen.getByRole("checkbox")).toBeDisabled();
      expect(start).toHaveValue(from);
      expect(end).toHaveValue(to);
    }
    expect(fetchTrackCutPreview).not.toHaveBeenCalled();
  });

  it("aborts a pending preview when discarded", async () => {
    vi.mocked(fetchTrackCutPreview).mockImplementation(
      () => new Promise(() => {}),
    );
    render(TrackCutEditor, { track, onChange: vi.fn() });
    await fireEvent.click(
      screen.getByRole("button", { name: "Select section" }),
    );
    await screen.findByLabelText("Remove from");
    await fireEvent.click(screen.getByRole("button", { name: "Preview join" }));
    const signal = vi.mocked(fetchTrackCutPreview).mock.calls[0][3];
    await fireEvent.click(screen.getByRole("button", { name: "Discard cut" }));
    await waitFor(() => expect(signal?.aborted).toBe(true));
    expect(
      screen.getByRole("button", { name: "Select section" }),
    ).toBeInTheDocument();
  });
});
