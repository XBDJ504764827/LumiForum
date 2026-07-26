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
