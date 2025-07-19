import { toast } from "svelte-sonner";

export interface ApiErrorOptions {
  showToast?: boolean;
  toastMessage?: string;
  logError?: boolean;
  context?: string;
}

export async function handleApiResponse<T>(
  response: Response,
  options: ApiErrorOptions = {},
): Promise<T> {
  const {
    showToast = false,
    toastMessage,
    logError = true,
    context = "API request",
  } = options;

  if (!response.ok) {
    let errorMessage = `HTTP error! status: ${response.status}`;

    try {
      const errorData = await response.json();
      errorMessage = errorData.detail || errorMessage;
    } catch {
      // Failed to parse error response, use default message
    }

    if (logError) {
      console.error(`Error in ${context}:`, errorMessage);
    }

    if (showToast) {
      toast.error(toastMessage || errorMessage);
    }

    throw new Error(errorMessage);
  }

  return await response.json();
}

export async function safeApiCall<T>(
  apiCall: () => Promise<T>,
  options: ApiErrorOptions = {},
): Promise<T | null> {
  const {
    showToast = false,
    toastMessage = "An error occurred",
    logError = true,
    context = "API call",
  } = options;

  try {
    return await apiCall();
  } catch (error) {
    if (logError) {
      console.error(`Error in ${context}:`, error);
    }

    if (showToast) {
      toast.error(toastMessage);
    }

    return null;
  }
}

export function createFormData(
  data: Record<string, string | File | boolean>,
): FormData {
  const formData = new FormData();

  for (const [key, value] of Object.entries(data)) {
    if (typeof value === "boolean") {
      formData.append(key, value.toString());
    } else {
      formData.append(key, value);
    }
  }

  return formData;
}
