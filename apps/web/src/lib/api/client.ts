import { getApiBaseUrl, joinUrl } from "@lumiforum/shared";
import type { ApiResponse } from "@lumiforum/types";

import { apiError, parseJson } from "@/lib/api/errors";
import { sessionAccessToken } from "@/lib/auth/session";

const DEFAULT_TIMEOUT_MS = 30_000;

export async function apiRequest<T>(
  path: string,
  init: RequestInit = {},
  authenticated = false,
  retryAfterRefresh = true,
): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.body && !(init.body instanceof FormData)) {
    headers.set("content-type", "application/json");
  }

  let hadBearer = false;
  if (authenticated) {
    headers.set("authorization", `Bearer ${await sessionAccessToken()}`);
    hadBearer = true;
  } else if (init.headers && hasAuthorization(headers)) {
    // The call site attached its own token (e.g. optionalAuthHeaders) — the
    // request is "authenticated" in effect, so a 401 must trigger a refresh.
    hadBearer = true;
  }

  const response = await fetchWithTimeout(path, init, headers);

  if (response.status === 401 && hadBearer && retryAfterRefresh) {
    headers.set("authorization", `Bearer ${await sessionAccessToken(true)}`);
    return apiRequest<T>(path, { ...init, headers }, true, false);
  }
  if (!response.ok) {
    throw await apiError(response);
  }
  const body = (await parseJson(response)) as ApiResponse<T>;
  return body.data;
}

/** fetch with an AbortController timeout so hung requests cannot pile up on
 * weak networks. Callers may pass their own `signal` in `init`; the timeout
 * aborts only when the caller did not provide one. */
async function fetchWithTimeout(
  path: string,
  init: RequestInit,
  headers: Headers,
): Promise<Response> {
  const url = joinUrl(getApiBaseUrl({ isServer: false }), path);
  const { signal, ...rest } = init;
  if (signal) {
    return fetch(url, { ...rest, headers, credentials: "include", signal });
  }

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT_MS);
  try {
    return await fetch(url, {
      ...rest,
      headers,
      credentials: "include",
      signal: controller.signal,
    });
  } finally {
    clearTimeout(timer);
  }
}

function hasAuthorization(headers: Headers): boolean {
  return Boolean(headers.get("authorization"));
}
