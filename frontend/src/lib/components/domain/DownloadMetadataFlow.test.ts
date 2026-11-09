import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import "@testing-library/jest-dom/vitest";
import DownloadManager from "./DownloadManager.svelte";
import DownloadReviewModal from "./DownloadReviewModal.svelte";
import { downloadStore } from "$lib/stores/downloadStore";
import { permissionsStore } from "$lib/stores/permissionsStore";
import { trackStore } from "$lib/stores/trackStore";
import * as apiClient from "$lib/services/apiClient";
import { handleMusEvent } from "$lib/services/eventHandlerService";
import { get } from "svelte/store";
import { existingTitleSuggestions } from "./trackMetadataForm";

vi.mock("$lib/services/apiClient", async (importOriginal) => ({
  ...(await importOriginal<typeof apiClient>()),
  fetchMetadata: vi.fn(),
  confirmDownload: vi.fn(),
  fetchTracks: vi.fn().mockResolvedValue([]),
}));

function renderDownload() {
  render(DownloadManager);
  render(DownloadReviewModal);
}

describe("download metadata review", () => {
  beforeEach(() => {
    downloadStore.reset();
    trackStore.reset();
    permissionsStore.set({ can_write_music_files: true });
    apiClient.trackUpdatesConnectionStatus.set("connected");
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
    renderDownload();
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
    await waitFor(() =>
      expect(
        screen.getByPlaceholderText(
          "Enter YouTube URL or other supported link",
        ),
      ).toHaveValue(""),
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
    renderDownload();
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
    renderDownload();
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
    renderDownload();
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
    renderDownload();
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

  it("waits for live updates before confirming a download", async () => {
    apiClient.trackUpdatesConnectionStatus.set("disconnected");
    renderDownload();
    await fireEvent.input(
      screen.getByPlaceholderText("Enter YouTube URL or other supported link"),
      { target: { value: "https://youtu.be/dQw4w9WgXcQ" } },
    );
    await fireEvent.click(screen.getByRole("button", { name: "Download" }));
    await screen.findByLabelText("Title");
    const save = screen.getByRole("button", { name: "Save" });
    expect(save).toBeDisabled();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Waiting for live updates",
    );
    await fireEvent.click(save);
    expect(apiClient.confirmDownload).not.toHaveBeenCalled();

    apiClient.trackUpdatesConnectionStatus.set("connected");
    await waitFor(() => expect(save).toBeEnabled());
    apiClient.trackUpdatesConnectionStatus.set("reconnecting");
    await waitFor(() => expect(save).toBeDisabled());
    apiClient.trackUpdatesConnectionStatus.set("connected");
    await waitFor(() => expect(save).toBeEnabled());
    await fireEvent.click(save);
    expect(apiClient.confirmDownload).toHaveBeenCalledOnce();
  });

  it("tracks an accepted download after the review is dismissed", async () => {
    let resolve!: () => void;
    vi.mocked(apiClient.confirmDownload).mockReturnValue(
      new Promise<void>((done) => {
        resolve = done;
      }),
    );
    renderDownload();
    const input = screen.getByPlaceholderText(
      "Enter YouTube URL or other supported link",
    );
    await fireEvent.input(input, {
      target: { value: "https://youtu.be/dQw4w9WgXcQ" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Download" }));
    await fireEvent.click(await screen.findByRole("button", { name: "Save" }));

    await fireEvent.keyDown(document, { key: "Escape" });
    await waitFor(() =>
      expect(screen.queryByLabelText("Title")).not.toBeInTheDocument(),
    );
    expect(get(downloadStore).state).toBe("awaiting_review");

    apiClient.trackUpdatesConnectionStatus.set("reconnecting");
    resolve();
    await waitFor(() => expect(get(downloadStore).state).toBe("downloading"));
    await waitFor(() => expect(input).toHaveValue(""));
  });

  it("releases a dismissed review if confirmation fails", async () => {
    let reject!: (error: Error) => void;
    vi.mocked(apiClient.confirmDownload).mockReturnValue(
      new Promise<void>((_, fail) => {
        reject = fail;
      }),
    );
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    renderDownload();
    const input = screen.getByPlaceholderText(
      "Enter YouTube URL or other supported link",
    );
    await fireEvent.input(input, {
      target: { value: "https://youtu.be/dQw4w9WgXcQ" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Download" }));
    await fireEvent.click(await screen.findByRole("button", { name: "Save" }));
    await fireEvent.keyDown(document, { key: "Escape" });
    reject(new Error("Confirmation failed"));

    await waitFor(() => expect(get(downloadStore).state).toBe("idle"));
    expect(input).toHaveValue("https://youtu.be/dQw4w9WgXcQ");
    log.mockRestore();
  });

  it("keeps a failed URL when another client starts a download", async () => {
    const url = "https://example.invalid/my-track";
    vi.mocked(apiClient.fetchMetadata).mockRejectedValueOnce(
      new Error("Metadata unavailable"),
    );
    renderDownload();
    const input = screen.getByPlaceholderText(
      "Enter YouTube URL or other supported link",
    );
    await fireEvent.input(input, { target: { value: url } });
    await fireEvent.click(screen.getByRole("button", { name: "Download" }));
    await waitFor(() => expect(get(downloadStore).state).toBe("failed"));

    handleMusEvent({
      action_key: "download_started",
      action_payload: { job_id: "another-client" },
      message_to_show: null,
      message_level: null,
    });
    await screen.findByRole("button", { name: "Downloading..." });
    expect(input).toHaveValue(url);
  });

  it("clears the same URL again when a failed download is retried", async () => {
    const url = "https://youtu.be/dQw4w9WgXcQ";
    renderDownload();
    const input = screen.getByPlaceholderText(
      "Enter YouTube URL or other supported link",
    );
    await fireEvent.input(input, { target: { value: url } });
    await fireEvent.click(screen.getByRole("button", { name: "Download" }));
    await fireEvent.click(await screen.findByRole("button", { name: "Save" }));
    await waitFor(() => expect(input).toHaveValue(""));

    handleMusEvent({
      action_key: "download_failed",
      action_payload: { error: "Failed" },
      message_to_show: null,
      message_level: null,
    });
    await screen.findByRole("button", { name: "Download" });
    await fireEvent.input(input, { target: { value: url } });
    await fireEvent.click(screen.getByRole("button", { name: "Download" }));
    await fireEvent.click(await screen.findByRole("button", { name: "Save" }));
    await waitFor(() => expect(input).toHaveValue(""));
    expect(apiClient.confirmDownload).toHaveBeenCalledTimes(2);
  });

  it.each([
    ["download_completed", "completed"],
    ["download_failed", "failed"],
  ] as const)(
    "preserves %s before confirmation returns",
    async (action, state) => {
      let resolve!: () => void;
      vi.mocked(apiClient.confirmDownload).mockReturnValue(
        new Promise<void>((done) => {
          resolve = done;
        }),
      );
      renderDownload();
      await fireEvent.input(
        screen.getByPlaceholderText(
          "Enter YouTube URL or other supported link",
        ),
        { target: { value: "https://youtu.be/dQw4w9WgXcQ" } },
      );
      await fireEvent.click(screen.getByRole("button", { name: "Download" }));
      await fireEvent.click(
        await screen.findByRole("button", { name: "Save" }),
      );
      const event = {
        message_to_show: null,
        message_level: null,
        action_payload: { error: "Failed" },
      };
      handleMusEvent({ ...event, action_key: "download_started" });
      handleMusEvent({ ...event, action_key: action });
      expect(get(downloadStore).state).toBe(state);
      resolve();
      await waitFor(() =>
        expect(screen.queryByLabelText("Title")).not.toBeInTheDocument(),
      );
      expect(get(downloadStore).state).toBe(state);
      expect(
        screen.getByPlaceholderText(
          "Enter YouTube URL or other supported link",
        ),
      ).toHaveValue("");
    },
  );

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
