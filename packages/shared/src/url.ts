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
  const isServer = options?.isServer ?? typeof window === "undefined";
  const publicUrl =
    options?.publicUrl ?? process.env.NEXT_PUBLIC_API_URL ?? defaultPublicApiUrl(isServer);
  const internalUrl = options?.internalUrl ?? process.env.API_INTERNAL_URL;

  if (isServer && internalUrl) {
    return stripTrailingSlash(internalUrl);
  }

  return stripTrailingSlash(publicUrl);
}

function defaultPublicApiUrl(isServer: boolean): string {
  if (!isServer && typeof window !== "undefined") {
    return `${window.location.protocol}//${window.location.hostname}:8080`;
  }
  return "http://localhost:8080";
}

export function joinUrl(base: string, path: string): string {
  const normalizedBase = stripTrailingSlash(base);
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  return `${normalizedBase}${normalizedPath}`;
}

function stripTrailingSlash(value: string): string {
  return value.endsWith("/") ? value.slice(0, -1) : value;
}
