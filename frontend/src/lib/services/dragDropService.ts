import { toast } from "svelte-sonner";
import { permissionsStore } from "$lib/stores/permissionsStore";
import { get } from "svelte/store";
import { analyzeAudioFile } from "$lib/utils/frontendCoverExtractor";
import {
  validateAudioFile,
  validateFileCount,
} from "$lib/utils/fileValidation";

export interface DragDropState {
  isDraggingFile: boolean;
  fileToUpload: File | null;
  uploadModalOpen: boolean;
}

import type {
  CoverInfo,
  AudioMetadata,
} from "$lib/utils/frontendCoverExtractor";

export interface ParsedFileInfo {
  file: File;
  suggestedTitle: string;
  suggestedArtist: string;
  coverInfo: CoverInfo | null;
  metadata: AudioMetadata;
  allTags: Record<string, unknown>;
}

export interface DragDropCallbacks {
  onDragStateChange: (isDragging: boolean) => void;
  onFileReady: (fileInfo: ParsedFileInfo) => void;
}

export class DragDropService {
  private callbacks: DragDropCallbacks;
  private state: DragDropState;

  constructor(callbacks: DragDropCallbacks) {
    this.callbacks = callbacks;
    this.state = {
      isDraggingFile: false,
      fileToUpload: null,
      uploadModalOpen: false,
    };
  }

  private parseFilename(filename: string): { title: string; artist: string } {
    // Remove file extension
    const nameWithoutExt = filename.replace(/\.[^/.]+$/, "");

    // Try to parse "Artist - Title" format
    const dashMatch = nameWithoutExt.match(/^(.+?)\s*-\s*(.+)$/);
    if (dashMatch) {
      return {
        artist: dashMatch[1].trim(),
        title: dashMatch[2].trim(),
      };
    }

    // Try to parse "Artist_Title" format
    const underscoreMatch = nameWithoutExt.match(/^(.+?)_(.+)$/);
    if (underscoreMatch) {
      return {
        artist: underscoreMatch[1].trim(),
        title: underscoreMatch[2].trim(),
      };
    }

    // If no pattern matches, use filename as title with empty artist
    return {
      artist: "",
      title: nameWithoutExt.trim(),
    };
  }

  private handleDragEnter = (event: DragEvent): void => {
    event.preventDefault();
    if (event.dataTransfer?.types.includes("Files")) {
      this.state.isDraggingFile = true;
      this.callbacks.onDragStateChange(true);
    }
  };

  private handleDragOver = (event: DragEvent): void => {
    event.preventDefault();
  };

  private handleDragLeave = (event: DragEvent): void => {
    // Only hide overlay if leaving the window entirely
    if (event.clientX === 0 && event.clientY === 0) {
      this.state.isDraggingFile = false;
      this.callbacks.onDragStateChange(false);
    }
  };

  private handleDrop = (event: DragEvent): void => {
    event.preventDefault();
    this.state.isDraggingFile = false;
    this.callbacks.onDragStateChange(false);

    const permissions = get(permissionsStore);
    if (!permissions.can_write_files) {
      toast.error(
        "File upload is not available - insufficient write permissions",
      );
      return;
    }

    const files = event.dataTransfer?.files;
    if (!validateFileCount(files || null)) {
      return;
    }

    const file = files![0];
    const validation = validateAudioFile(file);
    if (!validation.isValid) {
      return;
    }

    // Parse filename for artist and title suggestions
    const parsed = this.parseFilename(file.name);

    // Analyze audio file to extract comprehensive metadata and cover
    analyzeAudioFile(file)
      .then((analysis) => {
        // Use metadata from file if available, otherwise fall back to filename parsing
        const suggestedTitle = analysis.metadata.title || parsed.title;
        const suggestedArtist = analysis.metadata.artist || parsed.artist;

        // All checks passed, notify callback with parsed info
        this.callbacks.onFileReady({
          file,
          suggestedTitle,
          suggestedArtist,
          coverInfo: analysis.coverInfo,
          metadata: analysis.metadata,
          allTags: analysis.allTags,
        });
      })
      .catch((error) => {
        console.warn(
          "Error analyzing audio file, proceeding with filename parsing:",
          error,
        );
        // Proceed with filename parsing if analysis fails
        this.callbacks.onFileReady({
          file,
          suggestedTitle: parsed.title,
          suggestedArtist: parsed.artist,
          coverInfo: null,
          metadata: {},
          allTags: {},
        });
      });
  };

  public setupEventListeners(): void {
    if (typeof window !== "undefined") {
      window.addEventListener("dragenter", this.handleDragEnter);
      window.addEventListener("dragover", this.handleDragOver);
      window.addEventListener("dragleave", this.handleDragLeave);
      window.addEventListener("drop", this.handleDrop);
    }
  }

  public removeEventListeners(): void {
    if (typeof window !== "undefined") {
      window.removeEventListener("dragenter", this.handleDragEnter);
      window.removeEventListener("dragover", this.handleDragOver);
      window.removeEventListener("dragleave", this.handleDragLeave);
      window.removeEventListener("drop", this.handleDrop);
    }
  }

  public getState(): DragDropState {
    return { ...this.state };
  }
}
