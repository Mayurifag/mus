import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";
import DownloadManager from "./DownloadManager.svelte";
import { downloadStore } from "$lib/stores/downloadStore";
import { permissionsStore } from "$lib/stores/permissionsStore";
import * as apiClient from "$lib/services/apiClient";

// Mock dependencies
vi.mock("$lib/services/apiClient", () => ({
  startDownload: vi.fn(),
}));

vi.mock("svelte-sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock("svelte-sonner", () => ({
  toast: {
    error: vi.fn(),
  },
}));

describe("DownloadManager", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    downloadStore.reset();
    permissionsStore.set({ can_write_music_files: true });
  });

  it("should render download form", () => {
    render(DownloadManager);

    expect(screen.getByText("Download from URL")).toBeInTheDocument();
    expect(
      screen.getByPlaceholderText("Enter YouTube URL or other supported link"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /download/i }),
    ).toBeInTheDocument();
  });

  it("should disable button when no URL is entered", () => {
    render(DownloadManager);

    const button = screen.getByRole("button", { name: /download/i });
    expect(button).toBeDisabled();
  });

  it("should enable button when URL is entered", async () => {
    render(DownloadManager);

    const input = screen.getByPlaceholderText(
      "Enter YouTube URL or other supported link",
    );
    const button = screen.getByRole("button", { name: /download/i });

    await fireEvent.input(input, {
      target: { value: "https://youtube.com/watch?v=test" },
    });

    expect(button).not.toBeDisabled();
  });

  it("should call startDownload when form is submitted", async () => {
    const mockStartDownload = vi.mocked(apiClient.startDownload);
    mockStartDownload.mockResolvedValue();

    render(DownloadManager);

    const input = screen.getByPlaceholderText(
      "Enter YouTube URL or other supported link",
    );
    const button = screen.getByRole("button", { name: /download/i });

    await fireEvent.input(input, {
      target: { value: "https://youtube.com/watch?v=test" },
    });
    await fireEvent.click(button);

    expect(mockStartDownload).toHaveBeenCalledWith(
      "https://youtube.com/watch?v=test",
    );
  });

  it("should clear input after successful download start", async () => {
    const mockStartDownload = vi.mocked(apiClient.startDownload);
    mockStartDownload.mockResolvedValue();

    render(DownloadManager);

    const input = screen.getByPlaceholderText(
      "Enter YouTube URL or other supported link",
    ) as HTMLInputElement;
    const button = screen.getByRole("button", { name: /download/i });

    await fireEvent.input(input, {
      target: { value: "https://youtube.com/watch?v=test" },
    });
    await fireEvent.click(button);

    await waitFor(() => {
      expect(input.value).toBe("");
    });
  });

  it("should show loading state when downloading", async () => {
    downloadStore.startDownload();

    render(DownloadManager);

    expect(screen.getByText("Downloading...")).toBeInTheDocument();
    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("should show error message when download fails", async () => {
    downloadStore.setFailed("Network error");

    render(DownloadManager);

    expect(screen.getByText("Network error")).toBeInTheDocument();
  });

  it("should handle Enter key press", async () => {
    const mockStartDownload = vi.mocked(apiClient.startDownload);
    mockStartDownload.mockResolvedValue();

    render(DownloadManager);

    const input = screen.getByPlaceholderText(
      "Enter YouTube URL or other supported link",
    );

    await fireEvent.input(input, {
      target: { value: "https://youtube.com/watch?v=test" },
    });
    await fireEvent.keyDown(input, { key: "Enter" });

    expect(mockStartDownload).toHaveBeenCalledWith(
      "https://youtube.com/watch?v=test",
    );
  });

  it("should trim whitespace from URL", async () => {
    const mockStartDownload = vi.mocked(apiClient.startDownload);
    mockStartDownload.mockResolvedValue();

    render(DownloadManager);

    const input = screen.getByPlaceholderText(
      "Enter YouTube URL or other supported link",
    );
    const button = screen.getByRole("button", { name: /download/i });

    await fireEvent.input(input, {
      target: { value: "  https://youtube.com/watch?v=test  " },
    });
    await fireEvent.click(button);

    expect(mockStartDownload).toHaveBeenCalledWith(
      "https://youtube.com/watch?v=test",
    );
  });

  it("should show specific error message for rate limiting", async () => {
    const mockStartDownload = vi.mocked(apiClient.startDownload);
    mockStartDownload.mockRejectedValue(new Error("Too Many Requests"));

    render(DownloadManager);

    const input = screen.getByPlaceholderText(
      "Enter YouTube URL or other supported link",
    );
    const button = screen.getByRole("button", { name: /download/i });

    await fireEvent.input(input, {
      target: { value: "https://youtube.com/watch?v=test" },
    });
    await fireEvent.click(button);

    await waitFor(() => {
      expect(
        screen.getByText(
          "Too many download requests. Please wait a moment and try again.",
        ),
      ).toBeInTheDocument();
    });
  });

  it("should show specific error message for download in progress", async () => {
    const mockStartDownload = vi.mocked(apiClient.startDownload);
    mockStartDownload.mockRejectedValue(
      new Error("Download already in progress"),
    );

    render(DownloadManager);

    const input = screen.getByPlaceholderText(
      "Enter YouTube URL or other supported link",
    );
    const button = screen.getByRole("button", { name: /download/i });

    await fireEvent.input(input, {
      target: { value: "https://youtube.com/watch?v=test" },
    });
    await fireEvent.click(button);

    await waitFor(() => {
      expect(
        screen.getByText(
          "A download is already in progress. Please wait for it to complete.",
        ),
      ).toBeInTheDocument();
    });
  });

  it("should show specific error message for service unavailable", async () => {
    const mockStartDownload = vi.mocked(apiClient.startDownload);
    mockStartDownload.mockRejectedValue(new Error("Service Unavailable"));

    render(DownloadManager);

    const input = screen.getByPlaceholderText(
      "Enter YouTube URL or other supported link",
    );
    const button = screen.getByRole("button", { name: /download/i });

    await fireEvent.input(input, {
      target: { value: "https://youtube.com/watch?v=test" },
    });
    await fireEvent.click(button);

    await waitFor(() => {
      expect(
        screen.getByText(
          "Download service is temporarily unavailable. Please try again later.",
        ),
      ).toBeInTheDocument();
    });
  });

  it("should show completed state", async () => {
    render(DownloadManager);

    downloadStore.setCompleted();

    await waitFor(() => {
      expect(screen.getByText("Completed")).toBeInTheDocument();
    });
  });

  describe("Permission-based rendering", () => {
    it("should render download UI when write permissions are available", () => {
      permissionsStore.set({ can_write_music_files: true });

      render(DownloadManager);

      expect(screen.getByText("Download from URL")).toBeInTheDocument();
      expect(
        screen.getByPlaceholderText(
          "Enter YouTube URL or other supported link",
        ),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: /download/i }),
      ).toBeInTheDocument();
    });

    it("should show permission denied message when write permissions are not available", () => {
      permissionsStore.set({ can_write_music_files: false });

      render(DownloadManager);

      expect(screen.getByText("Download from URL")).toBeInTheDocument();
      expect(
        screen.getByText(
          "Download not available - music directory is read-only",
        ),
      ).toBeInTheDocument();
      expect(
        screen.queryByPlaceholderText(
          "Enter YouTube URL or other supported link",
        ),
      ).not.toBeInTheDocument();
      expect(
        screen.queryByRole("button", { name: /download/i }),
      ).not.toBeInTheDocument();
    });

    it("should hide input and button when permissions are revoked", async () => {
      // Start with permissions
      permissionsStore.set({ can_write_music_files: true });
      const { rerender } = render(DownloadManager);

      expect(
        screen.getByPlaceholderText(
          "Enter YouTube URL or other supported link",
        ),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: /download/i }),
      ).toBeInTheDocument();

      // Revoke permissions
      permissionsStore.set({ can_write_music_files: false });
      rerender({});

      await waitFor(() => {
        expect(
          screen.getByText(
            "Download not available - music directory is read-only",
          ),
        ).toBeInTheDocument();
      });

      expect(
        screen.queryByPlaceholderText(
          "Enter YouTube URL or other supported link",
        ),
      ).not.toBeInTheDocument();
      expect(
        screen.queryByRole("button", { name: /download/i }),
      ).not.toBeInTheDocument();
    });

    it("should show download UI when permissions are granted", async () => {
      // Start without permissions
      permissionsStore.set({ can_write_music_files: false });
      const { rerender } = render(DownloadManager);

      expect(
        screen.getByText(
          "Download not available - music directory is read-only",
        ),
      ).toBeInTheDocument();

      // Grant permissions
      permissionsStore.set({ can_write_music_files: true });
      rerender({});

      await waitFor(() => {
        expect(
          screen.getByPlaceholderText(
            "Enter YouTube URL or other supported link",
          ),
        ).toBeInTheDocument();
      });

      expect(
        screen.getByRole("button", { name: /download/i }),
      ).toBeInTheDocument();
      expect(
        screen.queryByText(
          "Download not available - music directory is read-only",
        ),
      ).not.toBeInTheDocument();
    });

    it("should show lock icon in permission denied message", () => {
      permissionsStore.set({ can_write_music_files: false });

      render(DownloadManager);

      const lockIcon = screen.getByTestId("lock-icon");
      expect(lockIcon).toBeInTheDocument();
    });
  });
});
