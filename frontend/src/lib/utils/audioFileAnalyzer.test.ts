import { vi, describe, it, expect, beforeEach, afterEach } from "vitest";
import {
  analyzeAudioFile,
  extractCoverFromFile,
  releaseCoverDataUrl,
} from "./audioFileAnalyzer";

// Mock MP3Tag
const mockMP3Tag = {
  read: vi.fn(),
  tags: {},
};

vi.mock("mp3tag.js", () => ({
  default: vi.fn(() => mockMP3Tag),
}));

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

// Mock Uint8Array
global.Uint8Array = vi
  .fn()
  .mockImplementation((data) => data) as unknown as typeof Uint8Array;

describe("audioFileAnalyzer", () => {
  const mockFile = new File(["test content"], "test.mp3", {
    type: "audio/mpeg",
  });
  const originalConsoleWarn = console.warn;

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
      const mockTags = {
        v1: {
          title: "V1 Title",
          artist: "V1 Artist",
          album: "V1 Album",
        },
        v2: {
          TIT2: "V2 Title",
          TPE1: "V2 Artist",
          TALB: "V2 Album",
          APIC: [
            {
              data: [1, 2, 3, 4],
              format: "image/jpeg",
              description: "Front Cover",
            },
          ],
          version: "2.4",
          flags: { unsynchronisation: false },
        },
      };

      mockMP3Tag.tags = mockTags;
      mockFile.arrayBuffer = vi.fn().mockResolvedValue(new ArrayBuffer(8));

      const result = await analyzeAudioFile(mockFile);

      expect(result.metadata.title).toBe("V2 Title");
      expect(result.metadata.artist).toBe("V2 Artist");
      expect(result.metadata.album).toBe("V2 Album");
      expect(result.coverInfo).toEqual({
        dataUrl: "blob:test-url",
        format: "image/jpeg",
        description: "Front Cover",
      });
      expect(result.allTags.v1).toEqual(mockTags.v1);
      expect(result.allTags.v2).not.toHaveProperty("version");
      expect(result.allTags.v2).not.toHaveProperty("flags");
    });

    it("should fallback to v1 tags when v2 tags are missing", async () => {
      const mockTags = {
        v1: {
          title: "V1 Title",
          artist: "V1 Artist",
          album: "V1 Album",
        },
        v2: {},
      };

      mockMP3Tag.tags = mockTags;
      mockFile.arrayBuffer = vi.fn().mockResolvedValue(new ArrayBuffer(8));

      const result = await analyzeAudioFile(mockFile);

      expect(result.metadata.title).toBe("V1 Title");
      expect(result.metadata.artist).toBe("V1 Artist");
      expect(result.metadata.album).toBe("V1 Album");
      expect(result.coverInfo).toBeNull();
    });

    it("should handle TPE2 fallback for artist when TPE1 is missing", async () => {
      const mockTags = {
        v1: {},
        v2: {
          TIT2: "Title",
          TPE2: "Album Artist",
          TALB: "Album",
        },
      };

      mockMP3Tag.tags = mockTags;
      mockFile.arrayBuffer = vi.fn().mockResolvedValue(new ArrayBuffer(8));

      const result = await analyzeAudioFile(mockFile);

      expect(result.metadata.artist).toBe("Album Artist");
    });

    it("should handle array values in v2 tags", async () => {
      const mockTags = {
        v1: {},
        v2: {
          TIT2: ["Array Title", "Second Title"],
          TPE1: ["Array Artist"],
          TALB: ["Array Album"],
        },
      };

      mockMP3Tag.tags = mockTags;
      mockFile.arrayBuffer = vi.fn().mockResolvedValue(new ArrayBuffer(8));

      const result = await analyzeAudioFile(mockFile);

      expect(result.metadata.title).toBe("Array Title");
      expect(result.metadata.artist).toBe("Array Artist");
      expect(result.metadata.album).toBe("Array Album");
    });

    it("should filter technical metadata from allTags", async () => {
      const mockTags = {
        v1: {},
        v2: {
          TIT2: "Title",
          version: "2.4",
          flags: { unsynchronisation: false },
          size: 1024,
          v2Details: { header: "info" },
          unsynchronisation: false,
          extendedHeader: true,
          experimentalIndicator: false,
          keepThis: "should remain",
        },
      };

      mockMP3Tag.tags = mockTags;
      mockFile.arrayBuffer = vi.fn().mockResolvedValue(new ArrayBuffer(8));

      const result = await analyzeAudioFile(mockFile);

      expect(result.allTags.v2).toHaveProperty("TIT2");
      expect(result.allTags.v2).toHaveProperty("keepThis");
      expect(result.allTags.v2).not.toHaveProperty("version");
      expect(result.allTags.v2).not.toHaveProperty("flags");
      expect(result.allTags.v2).not.toHaveProperty("size");
      expect(result.allTags.v2).not.toHaveProperty("v2Details");
    });

    it("should handle APIC data in allTags by replacing with byte count", async () => {
      const mockTags = {
        v1: {},
        v2: {
          APIC: [
            {
              data: [1, 2, 3, 4, 5],
              format: "image/jpeg",
              description: "Cover",
            },
            {
              data: [6, 7, 8],
              format: "image/png",
            },
          ],
        },
      };

      mockMP3Tag.tags = mockTags;
      mockFile.arrayBuffer = vi.fn().mockResolvedValue(new ArrayBuffer(8));

      const result = await analyzeAudioFile(mockFile);

      expect((result.allTags.v2 as Record<string, unknown>).APIC).toEqual([
        {
          data: "[5 bytes]",
          format: "image/jpeg",
          description: "Cover",
        },
        {
          data: "[3 bytes]",
          format: "image/png",
        },
      ]);
    });

    it("should handle missing v1 and v2 tags gracefully", async () => {
      const mockTags = {};

      mockMP3Tag.tags = mockTags;
      mockFile.arrayBuffer = vi.fn().mockResolvedValue(new ArrayBuffer(8));

      const result = await analyzeAudioFile(mockFile);

      expect(result.metadata).toEqual({
        title: undefined,
        artist: undefined,
        album: undefined,
      });
      expect(result.coverInfo).toBeNull();
      expect(result.allTags.v1).toBeUndefined();
      expect(result.allTags.v2).toEqual({});
    });

    it("should handle errors gracefully and return empty result", async () => {
      mockFile.arrayBuffer = vi
        .fn()
        .mockRejectedValue(new Error("File read error"));

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
      const mockTags = {
        v1: {},
        v2: {
          APIC: [
            {
              data: [1, 2, 3, 4],
              description: "Cover without format",
            },
          ],
        },
      };

      mockMP3Tag.tags = mockTags;
      mockFile.arrayBuffer = vi.fn().mockResolvedValue(new ArrayBuffer(8));

      const result = await analyzeAudioFile(mockFile);

      expect(result.coverInfo).toEqual({
        dataUrl: "blob:test-url",
        format: "image/jpeg",
        description: "Cover without format",
      });
    });

    it("should handle empty APIC array", async () => {
      const mockTags = {
        v1: {},
        v2: {
          APIC: [],
        },
      };

      mockMP3Tag.tags = mockTags;
      mockFile.arrayBuffer = vi.fn().mockResolvedValue(new ArrayBuffer(8));

      const result = await analyzeAudioFile(mockFile);

      expect(result.coverInfo).toBeNull();
    });
  });

  describe("extractCoverFromFile", () => {
    it("should extract cover info from file", async () => {
      const mockTags = {
        v1: {},
        v2: {
          APIC: [
            {
              data: [1, 2, 3, 4],
              format: "image/png",
              description: "Album Cover",
            },
          ],
        },
      };

      mockMP3Tag.tags = mockTags;
      mockFile.arrayBuffer = vi.fn().mockResolvedValue(new ArrayBuffer(8));

      const result = await extractCoverFromFile(mockFile);

      expect(result).toEqual({
        dataUrl: "blob:test-url",
        format: "image/png",
        description: "Album Cover",
      });
    });

    it("should return null when no cover exists", async () => {
      const mockTags = {
        v1: {},
        v2: {},
      };

      mockMP3Tag.tags = mockTags;
      mockFile.arrayBuffer = vi.fn().mockResolvedValue(new ArrayBuffer(8));

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
