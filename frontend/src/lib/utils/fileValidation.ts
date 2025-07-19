import { toast } from "svelte-sonner";

export const SUPPORTED_AUDIO_EXTENSIONS = ["mp3", "flac", "wav"];

export interface FileValidationOptions {
  showToast?: boolean;
  maxSizeMB?: number;
}

export interface FileValidationResult {
  isValid: boolean;
  error?: string;
  extension?: string;
}

export function validateAudioFile(
  file: File,
  options: FileValidationOptions = {},
): FileValidationResult {
  const { showToast = true, maxSizeMB } = options;

  // Get file extension
  const extension = file.name.toLowerCase().split(".").pop();

  // Validate extension
  if (!extension || !SUPPORTED_AUDIO_EXTENSIONS.includes(extension)) {
    const error =
      "Unsupported file format. Only MP3, FLAC, and WAV are supported.";
    if (showToast) toast.error(error);
    return { isValid: false, error };
  }

  // Validate file size
  const maxSize =
    maxSizeMB || parseInt(import.meta.env.VITE_MAX_UPLOAD_SIZE_MB || "30");
  const fileSizeMB = file.size / (1024 * 1024);

  if (fileSizeMB > maxSize) {
    const error = `File size (${fileSizeMB.toFixed(1)}MB) exceeds maximum allowed size (${maxSize}MB)`;
    if (showToast) toast.error(error);
    return { isValid: false, error };
  }

  return { isValid: true, extension };
}

export function validateFileCount(
  files: FileList | null,
  expectedCount: number = 1,
  showToast: boolean = true,
): boolean {
  if (!files || files.length !== expectedCount) {
    const error =
      expectedCount === 1
        ? "Please drop exactly one file"
        : `Please drop exactly ${expectedCount} files`;
    if (showToast) toast.error(error);
    return false;
  }
  return true;
}

export function getFileExtension(filename: string): string {
  return filename.toLowerCase().split(".").pop() || "";
}
