const INTERNAL_API_HOST = import.meta.env.VITE_INTERNAL_API_HOST || "";
export const PUBLIC_API_HOST = import.meta.env.VITE_PUBLIC_API_HOST || "";
const API_VERSION_PATH = "/api/v1";
export const API_BASE_URL = `${
  import.meta.env.SSR ? INTERNAL_API_HOST : PUBLIC_API_HOST
}${API_VERSION_PATH}`;
