/**
 * Resolve the public API base URL.
 * Browser: NEXT_PUBLIC_API_URL
 * Server (RSC / route handlers): API_INTERNAL_URL, then public fallback
 */
export function getApiBaseUrl(options?: {
  publicUrl?: string;
  internalUrl?: string;
  isServer?: boolean;
}): string {
  const publicUrl =
    options?.publicUrl ?? process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";
  const internalUrl = options?.internalUrl ?? process.env.API_INTERNAL_URL;
  const isServer = options?.isServer ?? typeof window === "undefined";

  if (isServer && internalUrl) {
    return stripTrailingSlash(internalUrl);
  }

  return stripTrailingSlash(publicUrl);
}

export function joinUrl(base: string, path: string): string {
  const normalizedBase = stripTrailingSlash(base);
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  return `${normalizedBase}${normalizedPath}`;
}

function stripTrailingSlash(value: string): string {
  return value.endsWith("/") ? value.slice(0, -1) : value;
}
