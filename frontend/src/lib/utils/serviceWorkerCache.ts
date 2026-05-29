export function isStaticAsset(pathname: string): boolean {
  return pathname.startsWith("/_app/immutable/");
}

export function shouldSkipCache(pathname: string): boolean {
  return pathname.startsWith("/api/");
}

export function isAudioStream(pathname: string): boolean {
  return /^\/api\/v1\/tracks\/\d+\/stream$/.test(pathname);
}
