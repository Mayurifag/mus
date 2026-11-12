import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/svelte";
import "@testing-library/jest-dom/vitest";
import TrackMetadataModal from "./TrackMetadataModal.svelte";
import { permissionsStore } from "$lib/stores/permissionsStore";
import { trackStore } from "$lib/stores/trackStore";
import { get } from "svelte/store";
import type { Track } from "$lib/types";
import * as apiClient from "$lib/services/trackClient";
import { toast } from "svelte-sonner";

vi.mock("$lib/services/trackClient", async (importOriginal) => ({
  ...(await importOriginal<typeof apiClient>()),
  updateTrack: vi.fn().mockResolvedValue({ status: "queued" }),
  uploadTrack: vi
    .fn()
    .mockResolvedValue({ success: true, message: "Uploaded" }),
}));

vi.mock("$lib/services/artworkClient", () => ({
  preloadArtworkSearch: vi.fn(() => () => {}),
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
  afterEach(() => vi.restoreAllMocks());
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

  it("leaves queued edits to live updates instead of changing the local track", async () => {
    trackStore.reset();
    trackStore.setTracks([track]);
    try {
      render(TrackMetadataModal, {
        props: { open: true, mode: "edit", track },
      });
      await fireEvent.input(await screen.findByLabelText("Title"), {
        target: { value: "Queued title" },
      });
      await fireEvent.click(screen.getByRole("button", { name: "Save" }));
      await waitFor(() =>
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
      );
      expect(apiClient.updateTrack).toHaveBeenCalledWith(track.id, {
        title: "Queued title",
        rename_file: true,
      });
      expect(get(trackStore).tracks).toEqual([track]);
      expect(get(trackStore).currentTrack).toEqual(track);
    } finally {
      trackStore.reset();
    }
  });

  it("shows the retry instruction when the track changed before deletion", async () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    const message = "Track not found; refresh and try again";
    vi.spyOn(apiClient, "deleteTrack").mockRejectedValueOnce(
      new Error(message),
    );
    const errorToast = vi.spyOn(toast, "error").mockReturnValue("toast");
    render(TrackMetadataModal, { props: { open: true, mode: "edit", track } });
    await screen.findByLabelText("Title");
    await fireEvent.click(screen.getByTitle("Delete track"));
    const confirmation = await screen.findByRole("dialog", {
      name: "Are you sure?",
    });
    await fireEvent.click(
      within(confirmation).getByRole("button", { name: "Delete" }),
    );
    await waitFor(() => expect(errorToast).toHaveBeenCalledWith(message));
    expect(confirmation).toBeInTheDocument();
  });

  it("keeps an unchanged WAV filename and does not offer a false rename", async () => {
    const wav = {
      ...track,
      title: "Song",
      artist: "Artist",
      filename: "Artist - Song.wav",
    };
    render(TrackMetadataModal, {
      props: { open: true, mode: "edit", track: wav },
    });
    await screen.findByDisplayValue("Song");
    expect(screen.getByText("Artist - Song.wav")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
  });

  it("rejects a filename that fits in characters but exceeds the UTF-8 byte limit", async () => {
    render(TrackMetadataModal, { props: { open: true, mode: "edit", track } });
    await fireEvent.input(await screen.findByLabelText("Title"), {
      target: { value: "Я".repeat(130) },
    });
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    await fireEvent.click(
      screen.getByRole("checkbox", { name: "Rename file to match title" }),
    );
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
  });

  it("previews an upload's original extension without stripping punctuation from its tags", async () => {
    const file = new File(["audio"], "original.FLAC", { type: "audio/flac" });
    render(TrackMetadataModal, {
      props: {
        open: true,
        mode: "create",
        file,
        suggestedTitle: "Song?",
        suggestedArtist: "Artist/Name",
      },
    });
    await screen.findByDisplayValue("Song?");
    expect(screen.getByText("ArtistName - Song.flac")).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(apiClient.uploadTrack).toHaveBeenCalledWith(
        file,
        "Song?",
        "Artist/Name",
        undefined,
      ),
    );
  });

  it.each(["edit", "create"] as const)(
    "keeps %s inputs available after saving fails",
    async (mode) => {
      vi.spyOn(console, "error").mockImplementation(() => {});
      if (mode === "edit")
        vi.mocked(apiClient.updateTrack).mockRejectedValueOnce(
          new Error("Read-only music directory"),
        );
      else
        vi.mocked(apiClient.uploadTrack).mockRejectedValueOnce(
          new Error("Read-only music directory"),
        );
      const props =
        mode === "edit"
          ? { mode, track }
          : {
              mode,
              file: new File(["audio"], "song.wav"),
              suggestedTitle: "Song",
              suggestedArtist: "Artist",
            };
      render(TrackMetadataModal, { props: { open: true, ...props } });
      await fireEvent.input(await screen.findByLabelText("Title"), {
        target: { value: "Keep my edits?" },
      });
      await fireEvent.click(screen.getByRole("button", { name: "Save" }));
      await waitFor(() =>
        expect(screen.getByRole("button", { name: "Save" })).toBeEnabled(),
      );
      expect(screen.getByLabelText("Title")).toHaveValue("Keep my edits?");
      expect(screen.getByRole("dialog")).toBeInTheDocument();
    },
  );
});
