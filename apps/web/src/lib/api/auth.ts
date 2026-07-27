import type {
  AuthResponse,
  LoginRequest,
  ProfileUpdateRequest,
  RegisterRequest,
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

export async function logout(): Promise<void> {
  await apiRequest<{ message: string }>("/auth/logout", { method: "POST" });
}
