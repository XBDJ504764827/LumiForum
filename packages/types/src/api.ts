/** Shared API contracts — keep in sync with apps/api JSON responses. */

export interface HealthResponse {
  status: string;
  service: string;
  timestamp: string;
}

export interface ReadyResponse {
  status: string;
  postgres: string;
  redis: string;
  timestamp: string;
}

export interface ApiErrorBody {
  error: {
    code: string;
    message: string;
  };
}

export interface ApiResponse<T> {
  data: T;
}

export type UserStatus = "active" | "pending" | "suspended" | "disabled";

export interface RoleSummary {
  code: string;
  name: string;
}

export interface User {
  id: string;
  username: string;
  email: string;
  avatar: string | null;
  nickname: string | null;
  role: RoleSummary;
  status: UserStatus;
  email_verified: boolean;
  created_at: string;
  updated_at: string;
}

export interface RegisterRequest {
  username: string;
  email: string;
  password: string;
  nickname?: string;
}

export interface LoginRequest {
  identifier: string;
  password: string;
}

export interface AuthResponse {
  access_token: string;
  token_type: "Bearer";
  expires_in: number;
  user: User;
}

export interface TokenRefreshResponse {
  access_token: string;
  token_type: "Bearer";
  expires_in: number;
}

export interface ProfileUpdateRequest {
  avatar?: string | null;
  nickname?: string | null;
}
