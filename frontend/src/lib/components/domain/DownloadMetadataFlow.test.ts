import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import "@testing-library/jest-dom/vitest";
import DownloadManager from "./DownloadManager.svelte";
import { downloadStore } from "$lib/stores/downloadStore";
import { permissionsStore } from "$lib/stores/permissionsStore";
import { trackStore } from "$lib/stores/trackStore";
import * as apiClient from "$lib/services/apiClient";
import { existingTitleSuggestions } from "./trackMetadataForm";

vi.mock("$lib/services/apiClient", async (importOriginal) => ({
  ...(await importOriginal<typeof apiClient>()),
  fetchMetadata: vi.fn(),
  confirmDownload: vi.fn(),
}));

describe("download metadata review", () => {
  beforeEach(() => {
    downloadStore.reset();
    trackStore.reset();
    permissionsStore.set({ can_write_music_files: true });
    vi.mocked(apiClient.fetchMetadata).mockResolvedValue({
      title: "Song (AI cover)",
      artist: "Artist",
      thumbnail_url: null,
      duration: null,
      tags: [{ name: "ai-cover", display_name: "AI cover" }],
    });
    vi.mocked(apiClient.confirmDownload).mockResolvedValue(undefined);
  });

  it("allows category selection and preserves punctuation in the edited metadata", async () => {
    render(DownloadManager);
    await fireEvent.input(
      screen.getByPlaceholderText("Enter YouTube URL or other supported link"),
      {
        target: { value: "https://youtu.be/dQw4w9WgXcQ" },
      },
    );
    await fireEvent.click(screen.getByRole("button", { name: "Download" }));
    const aiCover = await screen.findByRole("checkbox", { name: "AI cover" });
    expect(aiCover).toBeChecked();
    expect(
      screen.getByText(
        "Source duration is unavailable; check the recording version manually.",
      ),
    ).toBeInTheDocument();
    await fireEvent.click(aiCover);
    await fireEvent.click(screen.getByRole("checkbox", { name: "OST" }));
    await fireEvent.input(screen.getByLabelText("Title"), {
      target: { value: "Edited Song?" },
    });
    await fireEvent.input(
      screen.getByPlaceholderText("Primary artist (required)"),
      {
        target: { value: "Edited/Artist" },
      },
    );
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(apiClient.confirmDownload).toHaveBeenCalledWith(
        "https://youtu.be/dQw4w9WgXcQ",
        "Edited Song?",
        "Edited/Artist",
        undefined,
        ["ost"],
      ),
    );
  });

  it("offers close existing artist names without silently replacing them", async () => {
    trackStore.setTracks([
      {
        id: "1",
        title: "Song",
        artist: "Artist",
        duration: 120,
        filename: "song.mp3",
        updated_at: 1,
        has_cover: false,
        cover_small_url: null,
        cover_original_url: null,
        hls_url: "/song.m3u8",
      },
    ]);
    vi.mocked(apiClient.fetchMetadata).mockResolvedValue({
      title: "Song",
      artist: "Artst",
      thumbnail_url: null,
      duration: null,
      tags: [],
    });
    render(DownloadManager);
    await fireEvent.input(
      screen.getByPlaceholderText("Enter YouTube URL or other supported link"),
      {
        target: { value: "https://youtu.be/dQw4w9WgXcQ" },
      },
    );
    await fireEvent.click(screen.getByRole("button", { name: "Download" }));
    const input = await screen.findByPlaceholderText(
      "Primary artist (required)",
    );
    expect(input).toHaveValue("Artst");
    await fireEvent.click(screen.getByRole("button", { name: "Artist (1)" }));
    expect(input).toHaveValue("Artist");
  });

  it("makes additional source credits visible and removable", async () => {
    vi.mocked(apiClient.fetchMetadata).mockResolvedValue({
      title: "Song",
      artist: "Artist; Guest",
      thumbnail_url: null,
      duration: 180,
      tags: [],
    });
    render(DownloadManager);
    await fireEvent.input(
      screen.getByPlaceholderText("Enter YouTube URL or other supported link"),
      { target: { value: "https://youtu.be/dQw4w9WgXcQ" } },
    );
    await fireEvent.click(screen.getByRole("button", { name: "Download" }));
    expect(
      await screen.findByText(
        "Multiple credits may include other contributors; check each one.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("3:00")).toBeInTheDocument();
    await fireEvent.click(
      screen.getByRole("button", { name: "Remove artist 2" }),
    );
    expect(
      screen.queryByText(
        "Multiple credits may include other contributors; check each one.",
      ),
    ).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole("button", { name: "Add artist" }));
    await fireEvent.input(screen.getByPlaceholderText("Additional artist"), {
      target: { value: "Another Guest" },
    });
    expect(
      screen.getByText(
        "Multiple credits may include other contributors; check each one.",
      ),
    ).toBeInTheDocument();
    await fireEvent.click(
      screen.getByRole("button", { name: "Remove artist 2" }),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(apiClient.confirmDownload).toHaveBeenCalledWith(
        "https://youtu.be/dQw4w9WgXcQ",
        "Song",
        "Artist",
        undefined,
        [],
      ),
    );
  });

  it("keeps commas inside artist names when saving separate credits", async () => {
    vi.mocked(apiClient.fetchMetadata).mockResolvedValue({
      title: "Song",
      artist: "Tyler, The Creator; Guest",
      thumbnail_url: null,
      duration: 180,
      tags: [],
    });
    render(DownloadManager);
    await fireEvent.input(
      screen.getByPlaceholderText("Enter YouTube URL or other supported link"),
      { target: { value: "https://youtu.be/dQw4w9WgXcQ" } },
    );
    await fireEvent.click(screen.getByRole("button", { name: "Download" }));
    expect(
      await screen.findByPlaceholderText("Primary artist (required)"),
    ).toHaveValue("Tyler, The Creator");
    expect(screen.getByPlaceholderText("Additional artist")).toHaveValue(
      "Guest",
    );
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(apiClient.confirmDownload).toHaveBeenCalledWith(
        "https://youtu.be/dQw4w9WgXcQ",
        "Song",
        "Tyler, The Creator; Guest",
        undefined,
        [],
      ),
    );
  });

  it("offers a title spelling only with matching artist, title, and duration", async () => {
    trackStore.setTracks([
      {
        id: "1",
        title: "Song",
        artist: "Artist",
        duration: 120,
        filename: "song.mp3",
        updated_at: 1,
        has_cover: false,
        cover_small_url: null,
        cover_original_url: null,
        hls_url: "/song.m3u8",
      },
    ]);
    vi.mocked(apiClient.fetchMetadata).mockResolvedValue({
      title: "SONG",
      artist: "Artist",
      thumbnail_url: null,
      duration: 121,
      tags: [],
    });
    render(DownloadManager);
    await fireEvent.input(
      screen.getByPlaceholderText("Enter YouTube URL or other supported link"),
      { target: { value: "https://youtu.be/dQw4w9WgXcQ" } },
    );
    await fireEvent.click(screen.getByRole("button", { name: "Download" }));
    const title = await screen.findByLabelText("Title");
    expect(title).toHaveValue("SONG");
    expect(
      screen.getByText(
        "Similar durations do not prove recordings are the same.",
      ),
    ).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("button", { name: "Song" }));
    expect(title).toHaveValue("Song");
  });

  it("does not suggest title spellings without matching recording evidence", () => {
    const track = {
      id: "1",
      title: "Song",
      artist: "Artist",
      duration: 120,
      filename: "song.mp3",
      updated_at: 1,
      has_cover: false,
      cover_small_url: null,
      cover_original_url: null,
      hls_url: "/song.m3u8",
    };
    expect(
      existingTitleSuggestions("SONG", [{ id: 1, value: "Artist" }], null, [
        track,
      ]),
    ).toEqual([]);
    expect(
      existingTitleSuggestions("SONG", [{ id: 1, value: "Artist" }], 130, [
        track,
      ]),
    ).toEqual([]);
    expect(
      existingTitleSuggestions("SONG", [{ id: 1, value: "Other" }], 120, [
        track,
      ]),
    ).toEqual([]);
    expect(
      existingTitleSuggestions(
        "SONG (Live)",
        [{ id: 1, value: "Artist" }],
        120,
        [track],
      ),
    ).toEqual([]);
    expect(
      existingTitleSuggestions(
        "SONG (Acoustic)",
        [{ id: 1, value: "Artist" }],
        120,
        [track],
      ),
    ).toEqual([]);
  });
});
