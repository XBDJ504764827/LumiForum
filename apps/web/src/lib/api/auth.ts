import { getApiBaseUrl, joinUrl } from "@lumiforum/shared";
import type {
  AuthResponse,
  LoginRequest,
  ProfileUpdateRequest,
  RegisterRequest,
  SteamAuthorizationResponse,
  SteamUnbindRequest,
  User,
} from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";

export { ApiClientError, errorMessage } from "@/lib/api/errors";

export function login(input: LoginRequest): Promise<AuthResponse> {
  return apiRequest<AuthResponse>("/auth/login", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export function register(input: RegisterRequest): Promise<AuthResponse> {
  return apiRequest<AuthResponse>("/auth/register", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export function getMe(): Promise<User> {
  return apiRequest<User>("/auth/me", { method: "GET" }, true);
}

export function updateProfile(input: ProfileUpdateRequest): Promise<User> {
  return apiRequest<User>("/users/profile", { method: "PATCH", body: JSON.stringify(input) }, true);
}

export function steamLoginUrl(): string {
  return joinUrl(getApiBaseUrl({ isServer: false }), "/auth/steam/login");
}

export function bindSteam(): Promise<SteamAuthorizationResponse> {
  return apiRequest<SteamAuthorizationResponse>("/auth/steam/bind", { method: "POST" }, true);
}

export function unbindSteam(input: SteamUnbindRequest): Promise<User> {
  return apiRequest<User>(
    "/auth/steam/unbind",
    { method: "DELETE", body: JSON.stringify(input) },
    true,
  );
}

export function syncSteam(): Promise<User> {
  return apiRequest<User>("/auth/steam/sync", { method: "POST" }, true);
}

export async function logout(): Promise<void> {
  await apiRequest<{ message: string }>("/auth/logout", { method: "POST" });
}
