export type RealtimeServerMessage = {
  type: string;
  timestamp: string;
  data: Record<string, unknown>;
};

export type RealtimeStatus = "idle" | "connecting" | "connected" | "disconnected";
