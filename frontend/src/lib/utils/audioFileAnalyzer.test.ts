import { vi, describe, it, expect, beforeEach, afterEach } from "vitest";
import {
  analyzeAudioFile,
  extractCoverFromFile,
  releaseCoverDataUrl,
} from "./audioFileAnalyzer";
import type { IAudioMetadata, IPicture } from "music-metadata";

// Mock music-metadata
vi.mock("music-metadata");

// Mock URL.createObjectURL and URL.revokeObjectURL
const mockCreateObjectURL = vi.fn();
const mockRevokeObjectURL = vi.fn();

Object.defineProperty(global, "URL", {
  value: {
    createObjectURL: mockCreateObjectURL,
    revokeObjectURL: mockRevokeObjectURL,
  },
  writable: true,
});

// Mock Blob
global.Blob = vi.fn().mockImplementation((data, options) => ({
  data,
  type: options?.type || "application/octet-stream",
})) as unknown as typeof Blob;

describe("audioFileAnalyzer", () => {
  const mockFile = new File(["test content"], "test.mp3", {
    type: "audio/mpeg",
  });
  const originalConsoleWarn = console.warn;

  const getMockParseBlob = async () => {
    const { parseBlob } = await import("music-metadata");
    return vi.mocked(parseBlob);
  };

  const createMockMetadata = (
    common: Partial<IAudioMetadata["common"]> = {},
  ): IAudioMetadata => ({
    common: {
      track: { no: null, of: null },
      disk: { no: null, of: null },
      movementIndex: { no: null, of: null },
      ...common,
    },
    format: {
      trackInfo: [],
      tagTypes: [],
    },
    native: {},
    quality: {
      warnings: [],
    },
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockCreateObjectURL.mockReturnValue("blob:test-url");
    console.warn = vi.fn();
  });

  afterEach(() => {
    console.warn = originalConsoleWarn;
  });

  describe("analyzeAudioFile", () => {
    it("should analyze file with complete metadata and cover", async () => {
      const mockParseBlob = await getMockParseBlob();

      const mockPicture: IPicture = {
        format: "image/jpeg",
        data: Buffer.from([1, 2, 3, 4]),
        description: "Front Cover",
        type: "Cover (front)",
      };

      const mockMetadata = createMockMetadata({
        title: "Test Title",
        artist: "Test Artist",
        album: "Test Album",
        picture: [mockPicture],
      });

      mockParseBlob.mockResolvedValue(mockMetadata);

      const result = await analyzeAudioFile(mockFile);

      expect(result.metadata.title).toBe("Test Title");
      expect(result.metadata.artist).toBe("Test Artist");
      expect(result.metadata.album).toBe("Test Album");
      expect(result.coverInfo).toEqual({
        dataUrl: "blob:test-url",
        format: "image/jpeg",
        description: "Front Cover",
      });
      expect(result.allTags.picture).toEqual([
        {
          format: "image/jpeg",
          data: "[4 bytes]",
          description: "Front Cover",
          type: "Cover (front)",
        },
      ]);
    });

    it("should fallback to albumartist when artist is missing", async () => {
      const mockParseBlob = await getMockParseBlob();

      const mockMetadata = createMockMetadata({
        title: "Test Title",
        albumartist: "Album Artist",
        album: "Test Album",
      });

      mockParseBlob.mockResolvedValue(mockMetadata);

      const result = await analyzeAudioFile(mockFile);

      expect(result.metadata.title).toBe("Test Title");
      expect(result.metadata.artist).toBe("Album Artist");
      expect(result.metadata.album).toBe("Test Album");
      expect(result.coverInfo).toBeNull();
    });

    it("should parse artist and title from filename when tags are missing", async () => {
      const mockParseBlob = await getMockParseBlob();

      const fileWithName = new File(
        ["test content"],
        "Artist One, Artist Two - Song Title.mp3",
        {
          type: "audio/mpeg",
        },
      );

      const mockMetadata = createMockMetadata({});

      mockParseBlob.mockResolvedValue(mockMetadata);

      const result = await analyzeAudioFile(fileWithName);

      expect(result.metadata.title).toBe("Song Title");
      expect(result.metadata.artist).toBe("Artist One, Artist Two");
      expect(result.coverInfo).toBeNull();
    });

    it("should parse title from filename when only title tag is missing", async () => {
      const mockParseBlob = await getMockParseBlob();

      const fileWithName = new File(
        ["test content"],
        "Some Artist - Great Song.flac",
        {
          type: "audio/flac",
        },
      );

      const mockMetadata = createMockMetadata({
        artist: "Tagged Artist",
        album: "Tagged Album",
      });

      mockParseBlob.mockResolvedValue(mockMetadata);

      const result = await analyzeAudioFile(fileWithName);

      expect(result.metadata.title).toBe("Great Song");
      expect(result.metadata.artist).toBe("Tagged Artist");
      expect(result.metadata.album).toBe("Tagged Album");
    });

    it("should handle picture data in allTags by replacing with byte count", async () => {
      const mockParseBlob = await getMockParseBlob();

      const mockPictures: IPicture[] = [
        {
          format: "image/jpeg",
          data: Buffer.from([1, 2, 3, 4, 5]),
          description: "Cover",
          type: "Cover (front)",
        },
        {
          format: "image/png",
          data: Buffer.from([6, 7, 8]),
          type: "Cover (back)",
        },
      ];

      const mockMetadata = createMockMetadata({
        title: "Test Title",
        picture: mockPictures,
      });

      mockParseBlob.mockResolvedValue(mockMetadata);

      const result = await analyzeAudioFile(mockFile);

      expect(result.allTags.picture).toEqual([
        {
          format: "image/jpeg",
          data: "[5 bytes]",
          description: "Cover",
          type: "Cover (front)",
        },
        {
          format: "image/png",
          data: "[3 bytes]",
          type: "Cover (back)",
        },
      ]);
    });

    it("should handle missing metadata gracefully", async () => {
      const mockParseBlob = await getMockParseBlob();

      const mockMetadata = createMockMetadata({});

      mockParseBlob.mockResolvedValue(mockMetadata);

      const result = await analyzeAudioFile(mockFile);

      expect(result.metadata.title).toBe("test");
      expect(result.metadata.artist).toBeUndefined();
      expect(result.metadata.album).toBeUndefined();
      expect(result.coverInfo).toBeNull();
    });

    it("should handle errors gracefully and return empty result", async () => {
      const mockParseBlob = await getMockParseBlob();

      mockParseBlob.mockRejectedValue(new Error("Parse error"));

      const result = await analyzeAudioFile(mockFile);

      expect(result).toEqual({
        coverInfo: null,
        metadata: {},
        allTags: {},
      });
      expect(console.warn).toHaveBeenCalledWith(
        "Failed to analyze audio file:",
        expect.any(Error),
      );
    });

    it("should handle cover without format", async () => {
      const mockParseBlob = await getMockParseBlob();

      const mockPicture: IPicture = {
        format: "",
        data: Buffer.from([1, 2, 3, 4]),
        description: "Cover without format",
      };

      const mockMetadata = createMockMetadata({
        picture: [mockPicture],
      });

      mockParseBlob.mockResolvedValue(mockMetadata);

      const result = await analyzeAudioFile(mockFile);

      expect(result.coverInfo).toEqual({
        dataUrl: "blob:test-url",
        format: "image/jpeg",
        description: "Cover without format",
      });
    });

    it("should handle empty picture array", async () => {
      const mockParseBlob = await getMockParseBlob();

      const mockMetadata = createMockMetadata({
        picture: [],
      });

      mockParseBlob.mockResolvedValue(mockMetadata);

      const result = await analyzeAudioFile(mockFile);

      expect(result.coverInfo).toBeNull();
    });

    it("should test different audio formats", async () => {
      const mockParseBlob = await getMockParseBlob();

      const flacFile = new File(["test content"], "test.flac", {
        type: "audio/flac",
      });

      const mockMetadata = createMockMetadata({
        title: "FLAC Title",
        artist: "FLAC Artist",
      });

      mockParseBlob.mockResolvedValue(mockMetadata);

      const result = await analyzeAudioFile(flacFile);

      expect(result.metadata.title).toBe("FLAC Title");
      expect(result.metadata.artist).toBe("FLAC Artist");
    });
  });

  describe("extractCoverFromFile", () => {
    it("should extract cover info from file", async () => {
      const mockParseBlob = await getMockParseBlob();

      const mockPicture: IPicture = {
        format: "image/png",
        data: Buffer.from([1, 2, 3, 4]),
        description: "Album Cover",
      };

      const mockMetadata = createMockMetadata({
        picture: [mockPicture],
      });

      mockParseBlob.mockResolvedValue(mockMetadata);

      const result = await extractCoverFromFile(mockFile);

      expect(result).toEqual({
        dataUrl: "blob:test-url",
        format: "image/png",
        description: "Album Cover",
      });
    });

    it("should return null when no cover exists", async () => {
      const mockParseBlob = await getMockParseBlob();

      const mockMetadata = createMockMetadata({});

      mockParseBlob.mockResolvedValue(mockMetadata);

      const result = await extractCoverFromFile(mockFile);

      expect(result).toBeNull();
    });
  });

  describe("releaseCoverDataUrl", () => {
    it("should revoke blob URLs", () => {
      const blobUrl = "blob:http://localhost/test-blob";

      releaseCoverDataUrl(blobUrl);

      expect(mockRevokeObjectURL).toHaveBeenCalledWith(blobUrl);
    });

    it("should not revoke non-blob URLs", () => {
      const httpUrl = "http://example.com/image.jpg";

      releaseCoverDataUrl(httpUrl);

      expect(mockRevokeObjectURL).not.toHaveBeenCalled();
    });

    it("should handle data URLs", () => {
      const dataUrl = "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQ";

      releaseCoverDataUrl(dataUrl);

      expect(mockRevokeObjectURL).not.toHaveBeenCalled();
    });
  });
});
