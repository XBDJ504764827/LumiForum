import { getApiBaseUrl, joinUrl } from "@lumiforum/shared";
import type { ApiResponse, TokenRefreshResponse } from "@lumiforum/types";

import { ApiClientError, apiError, parseJson } from "@/lib/api/errors";

type SessionListener = (hasAccessToken: boolean) => void;

let accessToken: string | null = null;
let expiresAt = 0;
let refreshPromise: Promise<string> | null = null;
let refreshTimer: ReturnType<typeof setTimeout> | null = null;
const listeners = new Set<SessionListener>();

export function setAccessToken(token: string, expiresIn: number): void {
  accessToken = token;
  expiresAt = Date.now() + expiresIn * 1_000;
  scheduleRefresh(expiresIn);
  emit(true);
}

export function clearAccessToken(): void {
  accessToken = null;
  expiresAt = 0;
  if (refreshTimer) {
    clearTimeout(refreshTimer);
    refreshTimer = null;
  }
  emit(false);
}

export async function sessionAccessToken(forceRefresh = false): Promise<string> {
  const nearExpiry = expiresAt <= Date.now() + 30_000;
  if (!forceRefresh && accessToken && !nearExpiry) {
    return accessToken;
  }
  return refreshAccessToken();
}

export function subscribeSession(listener: SessionListener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

async function refreshAccessToken(): Promise<string> {
  if (refreshPromise) {
    return refreshPromise;
  }

  refreshPromise = withCrossTabLock(performRefresh).finally(() => {
    refreshPromise = null;
  });

  return refreshPromise;
}

async function performRefresh(): Promise<string> {
  try {
    const response = await fetch(joinUrl(getApiBaseUrl({ isServer: false }), "/auth/refresh"), {
      method: "POST",
      credentials: "include",
    });
    if (response.status === 204) {
      throw new ApiClientError(401, "authentication_required", "请先登录");
    }
    if (!response.ok) {
      throw await apiError(response);
    }
    const body = (await parseJson(response)) as ApiResponse<TokenRefreshResponse>;
    setAccessToken(body.data.access_token, body.data.expires_in);
    return body.data.access_token;
  } catch (error) {
    clearAccessToken();
    throw error;
  }
}

async function withCrossTabLock(task: () => Promise<string>): Promise<string> {
  if (typeof navigator !== "undefined" && navigator.locks) {
    return await navigator.locks.request("lumiforum-auth-refresh", task);
  }
  return await task();
}

function scheduleRefresh(expiresIn: number): void {
  if (refreshTimer) {
    clearTimeout(refreshTimer);
  }
  const refreshLeadSeconds = Math.min(60, Math.max(5, Math.floor(expiresIn * 0.1)));
  const delay = Math.max(1_000, (expiresIn - refreshLeadSeconds) * 1_000);
  refreshTimer = setTimeout(() => {
    void refreshAccessToken().catch(() => undefined);
  }, delay);
}

function emit(hasAccessToken: boolean): void {
  for (const listener of listeners) {
    listener(hasAccessToken);
  }
}
