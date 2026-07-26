import { getApiBaseUrl, joinUrl } from "@lumiforum/shared";
import type { ApiResponse } from "@lumiforum/types";

import { apiError, parseJson } from "@/lib/api/errors";
import { sessionAccessToken } from "@/lib/auth/session";

export async function apiRequest<T>(
  path: string,
  init: RequestInit = {},
  authenticated = false,
  retryAfterRefresh = true,
): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.body) {
    headers.set("content-type", "application/json");
  }
  if (authenticated) {
    headers.set("authorization", `Bearer ${await sessionAccessToken()}`);
  }

  const response = await fetch(joinUrl(getApiBaseUrl({ isServer: false }), path), {
    ...init,
    headers,
    credentials: "include",
  });
  if (response.status === 401 && authenticated && retryAfterRefresh) {
    headers.set("authorization", `Bearer ${await sessionAccessToken(true)}`);
    return apiRequest<T>(path, { ...init, headers }, true, false);
  }
  if (!response.ok) {
    throw await apiError(response);
  }
  const body = (await parseJson(response)) as ApiResponse<T>;
  return body.data;
}
