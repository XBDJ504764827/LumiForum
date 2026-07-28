import type { PresenceStatus } from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";

export function getUserPresence(userId: string): Promise<PresenceStatus> {
  return apiRequest<PresenceStatus>(`/users/${encodeURIComponent(userId)}/presence`);
}
