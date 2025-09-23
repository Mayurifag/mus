import { parseBlob } from "music-metadata";
import type { IAudioMetadata, IPicture } from "music-metadata";

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

function extractCoverInfo(pictures?: IPicture[]): CoverInfo | null {
  if (pictures && pictures.length > 0) {
    const picture = pictures[0];
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

function parseFilenameForMetadata(filename: string): {
  artist?: string;
  title?: string;
} {
  const nameWithoutExt = filename.replace(/\.[^/.]+$/, "");
  const separatorIndex = nameWithoutExt.indexOf(" - ");

  if (separatorIndex === -1) {
    return { title: nameWithoutExt };
  }

  const artist = nameWithoutExt.substring(0, separatorIndex).trim();
  const title = nameWithoutExt.substring(separatorIndex + 3).trim();

  return { artist, title };
}

export async function analyzeAudioFile(
  file: File,
): Promise<FileAnalysisResult> {
  try {
    const metadata: IAudioMetadata = await parseBlob(file);
    const coverInfo = extractCoverInfo(metadata.common.picture);

    const { title: filenameTitle, artist: filenameArtist } =
      parseFilenameForMetadata(file.name);

    const commonMetadata: AudioMetadata = {
      title: metadata.common.title || filenameTitle,
      artist:
        metadata.common.artist || metadata.common.albumartist || filenameArtist,
      album: metadata.common.album,
    };

    const cleanedPictures = metadata.common.picture?.map((pic) => ({
      ...pic,
      data: `[${pic.data.length} bytes]`,
    }));

    const allTags: Record<string, unknown> = {
      ...metadata.common,
      picture: cleanedPictures,
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
