import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import "@testing-library/jest-dom/vitest";
import TrackMetadataModal from "./TrackMetadataModal.svelte";
import { permissionsStore } from "$lib/stores/permissionsStore";
import type { Track } from "$lib/types";
import * as apiClient from "$lib/services/apiClient";

vi.mock("$lib/services/apiClient", async (importOriginal) => ({
  ...(await importOriginal<typeof apiClient>()),
  preloadArtworkSearch: vi.fn(() => () => {}),
  updateTrack: vi.fn().mockResolvedValue({ status: "queued" }),
}));

const track: Track = {
  id: "1".repeat(64),
  title: "Edited Song?",
  artist: "Edited/Artist",
  filename: "EditedArtist - Edited Song.mp3",
  duration: 120,
  updated_at: 1,
  has_cover: false,
  cover_small_url: null,
  cover_original_url: null,
  hls_url: "/song.m3u8",
  tags: [{ name: "cover", display_name: "Cover" }],
};

describe("editing track metadata", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    permissionsStore.set({ can_write_music_files: true });
  });

  it("changes categories without changing punctuation in metadata", async () => {
    render(TrackMetadataModal, { props: { open: true, mode: "edit", track } });
    expect(await screen.findByLabelText("Title")).toHaveValue("Edited Song?");
    const save = screen.getByRole("button", { name: "Save" });
    expect(save).toBeDisabled();
    const cover = screen.getByRole("checkbox", { name: "Cover" });
    expect(cover).toBeChecked();
    await fireEvent.click(cover);
    await fireEvent.click(screen.getByRole("checkbox", { name: "OST" }));
    await fireEvent.click(save);
    await waitFor(() =>
      expect(apiClient.updateTrack).toHaveBeenCalledWith(track.id, {
        tags: ["ost"],
      }),
    );
  });

  it("preserves punctuation when editing metadata and renaming the file", async () => {
    render(TrackMetadataModal, { props: { open: true, mode: "edit", track } });
    await fireEvent.input(await screen.findByLabelText("Title"), {
      target: { value: "New? Name" },
    });
    await fireEvent.input(
      screen.getByPlaceholderText("Primary artist (required)"),
      { target: { value: "Artist/Name" } },
    );
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(apiClient.updateTrack).toHaveBeenCalledWith(track.id, {
        title: "New? Name",
        artist: "Artist/Name",
        rename_file: true,
      }),
    );
  });
});
