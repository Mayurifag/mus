import MP3Tag from "mp3tag.js";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type MP3TagData = any;

export interface CoverInfo {
  dataUrl: string;
  format: string;
  description?: string;
}

export interface AudioMetadata {
  title?: string;
  artist?: string;
  album?: string;
  [key: string]: unknown;
}

export interface FileAnalysisResult {
  coverInfo: CoverInfo | null;
  metadata: AudioMetadata;
  allTags: Record<string, unknown>;
}

function filterTechnicalMetadata(
  tags: Record<string, unknown>,
): Record<string, unknown> {
  const libraryKeys = [
    "v2Details",
    "version",
    "flags",
    "size",
    "unsynchronisation",
    "extendedHeader",
    "experimentalIndicator",
  ];

  const filtered: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(tags)) {
    if (!libraryKeys.includes(key)) {
      filtered[key] = value;
    }
  }

  return filtered;
}

function extractCoverInfo(tags: MP3TagData): CoverInfo | null {
  if (tags.v2 && tags.v2.APIC && tags.v2.APIC.length > 0) {
    const picture = tags.v2.APIC[0];
    const blob = new Blob([new Uint8Array(picture.data)], {
      type: picture.format || "image/jpeg",
    });
    const dataUrl = URL.createObjectURL(blob);

    return {
      dataUrl,
      format: picture.format || "image/jpeg",
      description: picture.description,
    };
  }
  return null;
}

function getTagValue(
  v2: Record<string, unknown>,
  v1: Record<string, unknown>,
  v2Key: string,
  v1Key?: string,
) {
  if (v2[v2Key]) {
    return Array.isArray(v2[v2Key]) ? v2[v2Key][0] : v2[v2Key];
  }
  if (v1Key && v1[v1Key]) {
    return v1[v1Key];
  }
  return undefined;
}



export async function analyzeAudioFile(
  file: File,
): Promise<FileAnalysisResult> {
  try {
    const buffer = await file.arrayBuffer();
    const mp3tag = new MP3Tag(buffer);
    mp3tag.read();

    const tags = mp3tag.tags;
    const coverInfo = extractCoverInfo(tags as MP3TagData);

    const v1 = (tags.v1 || {}) as Record<string, unknown>;
    const v2 = (tags.v2 || {}) as Record<string, unknown>;

    const commonMetadata: AudioMetadata = {
      title: getTagValue(v2, v1, "TIT2", "title")?.toString(),
      // Uses TPE1 (main artist) first, falls back to TPE2 (album artist) if not available
      artist: getTagValue(v2, v1, "TPE1", "artist")?.toString() || getTagValue(v2, v1, "TPE2")?.toString(),
      album: getTagValue(v2, v1, "TALB", "album")?.toString(),
    };

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

export async function extractCoverFromFile(
  file: File,
): Promise<CoverInfo | null> {
  const result = await analyzeAudioFile(file);
  return result.coverInfo;
}

export function releaseCoverDataUrl(dataUrl: string): void {
  if (dataUrl.startsWith("blob:")) {
    URL.revokeObjectURL(dataUrl);
  }
}
