export function isStaticAsset(pathname: string): boolean {
  return pathname.startsWith("/_app/immutable/");
}

export function shouldSkipCache(pathname: string): boolean {
  return pathname.startsWith("/api/");
}
