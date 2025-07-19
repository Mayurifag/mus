import MP3Tag from "mp3tag.js";

export interface CoverInfo {
  dataUrl: string;
  format: string;
  description?: string;
}

export interface AudioMetadata {
  title?: string;
  artist?: string;
  album?: string;
  albumartist?: string;
  date?: string;
  year?: number;
  track?: { no: number; of?: number };
  disk?: { no: number; of?: number };
  genre?: string[];
  comment?: string[];
  duration?: number;
  bitrate?: number;
  sampleRate?: number;
  bitsPerSample?: number;
  codec?: string;
  container?: string;
  tool?: string;
  encoder?: string;
  [key: string]: unknown;
}

export interface FileAnalysisResult {
  coverInfo: CoverInfo | null;
  metadata: AudioMetadata;
  allTags: Record<string, unknown>;
}

/**
 * Filter out technical metadata that users don't typically need to edit.
 */
function filterTechnicalMetadata(
  tags: Record<string, unknown>,
): Record<string, unknown> {
  const technicalKeys = [
    "v2Details",
    "version",
    "flags",
    "size",
    "unsynchronisation",
    "extendedHeader",
    "experimentalIndicator",
    "APIC", // Picture data
    "PRIV", // Private frames
    "UFID", // Unique file identifier
    "MCDI", // Music CD identifier
    "SYTC", // Synchronized tempo codes
    "USLT", // Unsynchronized lyrics
    "SYLT", // Synchronized lyrics
    "RVA2", // Relative volume adjustment
    "EQU2", // Equalization
    "RVRB", // Reverb
    "PCNT", // Play counter
    "POPM", // Popularimeter
    "RBUF", // Recommended buffer size
    "AENC", // Audio encryption
    "LINK", // Linked information
    "POSS", // Position synchronization
    "USER", // Terms of use
    "OWNE", // Ownership
    "COMR", // Commercial
    "ENCR", // Encryption method registration
    "GRID", // Group identification registration
    "SIGN", // Signature
  ];

  const filtered: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(tags)) {
    if (!technicalKeys.includes(key)) {
      filtered[key] = value;
    }
  }

  return filtered;
}

/**
 * Extract comprehensive metadata and cover art from an audio file.
 */
export async function analyzeAudioFile(
  file: File,
): Promise<FileAnalysisResult> {
  try {
    // Convert File to ArrayBuffer
    const buffer = await file.arrayBuffer();

    // Parse with mp3tag.js
    const mp3tag = new MP3Tag(buffer);
    mp3tag.read();

    // Extract cover art
    let coverInfo: CoverInfo | null = null;
    const tags = mp3tag.tags;
    if (tags.v2 && tags.v2.APIC && tags.v2.APIC.length > 0) {
      const picture = tags.v2.APIC[0];
      const blob = new Blob([new Uint8Array(picture.data)], {
        type: picture.format || "image/jpeg",
      });
      const dataUrl = URL.createObjectURL(blob);

      coverInfo = {
        dataUrl,
        format: picture.format || "image/jpeg",
        description: picture.description,
      };
    }

    // Extract metadata from ID3v1 and ID3v2 tags
    const v1 = (tags.v1 || {}) as Record<string, unknown>;
    const v2 = (tags.v2 || {}) as Record<string, unknown>;

    // Helper function to get tag value, preferring v2 over v1
    const getTag = (v2Key: string, v1Key?: string) => {
      if (v2[v2Key]) {
        return Array.isArray(v2[v2Key]) ? v2[v2Key][0] : v2[v2Key];
      }
      if (v1Key && v1[v1Key]) {
        return v1[v1Key];
      }
      return undefined;
    };

    // Parse track number
    let track: { no: number; of?: number } | undefined;
    const trackString = getTag("TRCK", "track");
    if (trackString) {
      const trackParts = trackString.toString().split("/");
      track = {
        no: parseInt(trackParts[0]) || 0,
        of: trackParts[1] ? parseInt(trackParts[1]) : undefined,
      };
    }

    // Parse year
    let year: number | undefined;
    const yearString =
      getTag("TYER") || getTag("TDRC") || getTag("TDAT", "year");
    if (yearString) {
      const parsed = parseInt(yearString.toString());
      if (!isNaN(parsed)) year = parsed;
    }

    // Parse genre
    let genre: string[] | undefined;
    const genreString = getTag("TCON", "genre");
    if (genreString) {
      genre = [genreString.toString()];
    }

    // Parse comment
    let comment: string[] | undefined;
    const commentString = getTag("COMM", "comment");
    if (commentString) {
      comment = [commentString.toString()];
    }

    // Try to get duration from TLEN tag (length in milliseconds) or calculate from file
    let duration: number | undefined;
    const tlenTag = getTag("TLEN");
    if (tlenTag) {
      const tlenMs = parseInt(tlenTag.toString());
      if (!isNaN(tlenMs)) {
        duration = Math.round(tlenMs / 1000); // Convert to seconds
      }
    }

    const commonMetadata: AudioMetadata = {
      title: getTag("TIT2", "title")?.toString(),
      artist: getTag("TPE1", "artist")?.toString(),
      album: getTag("TALB", "album")?.toString(),
      albumartist: getTag("TPE2")?.toString(),
      year,
      track,
      genre,
      comment,
      duration,
    };

    // Collect all raw tags for display (excluding picture data)
    const cleanV2: Record<string, unknown> = tags.v2 ? { ...tags.v2 } : {};
    if (cleanV2.APIC && Array.isArray(cleanV2.APIC)) {
      cleanV2.APIC = cleanV2.APIC.map((pic: unknown) => {
        const picObj = pic as Record<string, unknown>;
        return {
          ...picObj,
          data: `[${(picObj.data as unknown[] | undefined)?.length || 0} bytes]`,
        };
      });
    }

    // Filter out technical metadata from v2 tags
    const filteredV2 = filterTechnicalMetadata(cleanV2);

    const allTags: Record<string, unknown> = {
      v1: tags.v1,
      v2: filteredV2,
      raw: {
        ...mp3tag.tags,
        v2: filteredV2,
      },
    };

    return {
      coverInfo,
      metadata: commonMetadata,
      allTags,
    };
  } catch (error) {
    console.warn("Failed to analyze audio file:", error);
    return {
      coverInfo: null,
      metadata: {},
      allTags: {},
    };
  }
}

/**
 * Extract cover art from an audio file (legacy function for compatibility).
 */
export async function extractCoverFromFile(
  file: File,
): Promise<CoverInfo | null> {
  const result = await analyzeAudioFile(file);
  return result.coverInfo;
}

/**
 * Clean up a data URL created by extractCoverFromFile.
 * Call this when you're done with the cover image to free memory.
 */
export function releaseCoverDataUrl(dataUrl: string): void {
  if (dataUrl.startsWith("blob:")) {
    URL.revokeObjectURL(dataUrl);
  }
}
