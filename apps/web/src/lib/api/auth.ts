import { getApiBaseUrl, joinUrl } from "@lumiforum/shared";
import type {
  ApiResponse,
  AuthResponse,
  LoginRequest,
  ProfileUpdateRequest,
  RegisterRequest,
  User,
} from "@lumiforum/types";

import { apiError, parseJson } from "@/lib/api/errors";
import { sessionAccessToken } from "@/lib/auth/session";

export { ApiClientError, errorMessage } from "@/lib/api/errors";

export function login(input: LoginRequest): Promise<AuthResponse> {
  return request<AuthResponse>("/auth/login", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export function register(input: RegisterRequest): Promise<AuthResponse> {
  return request<AuthResponse>("/auth/register", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export function getMe(): Promise<User> {
  return request<User>("/auth/me", { method: "GET" }, true);
}

export function updateProfile(input: ProfileUpdateRequest): Promise<User> {
  return request<User>("/users/profile", { method: "PATCH", body: JSON.stringify(input) }, true);
}

export async function logout(): Promise<void> {
  await request<{ message: string }>("/auth/logout", { method: "POST" });
}

async function request<T>(
  path: string,
  init: RequestInit,
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
    return request<T>(path, { ...init, headers }, true, false);
  }
  if (!response.ok) {
    throw await apiError(response);
  }
  const body = (await parseJson(response)) as ApiResponse<T>;
  return body.data;
}
