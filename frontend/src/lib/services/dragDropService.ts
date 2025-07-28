import { toast } from "svelte-sonner";
import { permissionsStore } from "$lib/stores/permissionsStore";
import { get } from "svelte/store";
import { analyzeAudioFile } from "$lib/utils/audioFileAnalyzer";
import {
  validateAudioFile,
  validateFileCount,
} from "$lib/utils/fileValidation";

export interface DragDropState {
  isDraggingFile: boolean;
  fileToUpload: File | null;
  uploadModalOpen: boolean;
}

import type { CoverInfo, AudioMetadata } from "$lib/utils/audioFileAnalyzer";

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
    const nameWithoutExt = filename.replace(/\.[^/.]+$/, "");

    const dashMatch = nameWithoutExt.match(/^(.+?)\s*-\s*(.+)$/);
    if (dashMatch) {
      return {
        artist: dashMatch[1].trim(),
        title: dashMatch[2].trim(),
      };
    }

    const underscoreMatch = nameWithoutExt.match(/^(.+?)_(.+)$/);
    if (underscoreMatch) {
      return {
        artist: underscoreMatch[1].trim(),
        title: underscoreMatch[2].trim(),
      };
    }

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
    if (!permissions.can_write_music_files) {
      toast.error(
        "File upload is not available - music directory is read-only",
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

    const parsed = this.parseFilename(file.name);

    analyzeAudioFile(file)
      .then((analysis) => {
        const suggestedTitle = analysis.metadata.title || parsed.title;
        const suggestedArtist = analysis.metadata.artist || parsed.artist;

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
