import { buildWsUrl } from "@lumiforum/shared";

import type { RealtimeServerMessage, RealtimeStatus } from "@/lib/realtime/types";

type MessageHandler = (message: RealtimeServerMessage) => void;
type StatusHandler = (status: RealtimeStatus) => void;

const MAX_BACKOFF_MS = 15_000;
const BASE_BACKOFF_MS = 1_000;
const PING_INTERVAL_MS = 25_000;

export class RealtimeClient {
  private socket: WebSocket | null = null;
  private status: RealtimeStatus = "idle";
  private shouldRun = false;
  private reconnectAttempt = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private tokenProvider: (() => Promise<string>) | null = null;
  private readonly messageHandlers = new Set<MessageHandler>();
  private readonly statusHandlers = new Set<StatusHandler>();

  setTokenProvider(provider: () => Promise<string>): void {
    this.tokenProvider = provider;
  }

  onMessage(handler: MessageHandler): () => void {
    this.messageHandlers.add(handler);
    return () => this.messageHandlers.delete(handler);
  }

  onStatus(handler: StatusHandler): () => void {
    this.statusHandlers.add(handler);
    handler(this.status);
    return () => this.statusHandlers.delete(handler);
  }

  start(): void {
    if (this.shouldRun) return;
    this.shouldRun = true;
    void this.connect();
  }

  stop(): void {
    this.shouldRun = false;
    this.clearTimers();
    if (this.socket) {
      this.socket.close();
      this.socket = null;
    }
    this.setStatus("disconnected");
  }

  send(type: string, data: Record<string, unknown> = {}): void {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) return;
    this.socket.send(JSON.stringify({ type, data }));
  }

  private async connect(): Promise<void> {
    if (!this.shouldRun || !this.tokenProvider) return;
    this.setStatus("connecting");
    try {
      const token = await this.tokenProvider();
      const url = buildWsUrl(token);
      const socket = new WebSocket(url);
      this.socket = socket;

      socket.onopen = () => {
        this.reconnectAttempt = 0;
        this.setStatus("connected");
        this.startPing();
      };

      socket.onmessage = (event) => {
        try {
          const message = JSON.parse(String(event.data)) as RealtimeServerMessage;
          for (const handler of this.messageHandlers) handler(message);
        } catch {
          // ignore malformed frames
        }
      };

      socket.onerror = () => {
        // onclose will schedule reconnect
      };

      socket.onclose = () => {
        this.clearPing();
        this.socket = null;
        if (!this.shouldRun) {
          this.setStatus("disconnected");
          return;
        }
        this.setStatus("disconnected");
        this.scheduleReconnect();
      };
    } catch {
      this.setStatus("disconnected");
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect(): void {
    if (!this.shouldRun || this.reconnectTimer) return;
    const delay = Math.min(MAX_BACKOFF_MS, BASE_BACKOFF_MS * 2 ** this.reconnectAttempt);
    this.reconnectAttempt += 1;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      void this.connect();
    }, delay);
  }

  private startPing(): void {
    this.clearPing();
    this.pingTimer = setInterval(() => {
      this.send("ping");
    }, PING_INTERVAL_MS);
  }

  private clearPing(): void {
    if (this.pingTimer) {
      clearInterval(this.pingTimer);
      this.pingTimer = null;
    }
  }

  private clearTimers(): void {
    this.clearPing();
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private setStatus(status: RealtimeStatus): void {
    this.status = status;
    for (const handler of this.statusHandlers) handler(status);
  }
}
