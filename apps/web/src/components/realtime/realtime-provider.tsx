"use client";

import { useQueryClient } from "@tanstack/react-query";
import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import { useAuth } from "@/components/auth/auth-provider";
import { dmKeys } from "@/lib/api/dm";
import { notificationKeys } from "@/lib/api/notifications";
import { sessionAccessToken } from "@/lib/auth/session";
import { RealtimeClient } from "@/lib/realtime/client";
import type { RealtimeServerMessage, RealtimeStatus } from "@/lib/realtime/types";

interface RealtimeContextValue {
  status: RealtimeStatus;
  client: RealtimeClient | null;
}

const RealtimeContext = createContext<RealtimeContextValue>({
  status: "idle",
  client: null,
});

export function RealtimeProvider({ children }: { children: ReactNode }) {
  const { status: authStatus } = useAuth();
  const queryClient = useQueryClient();
  const [socketStatus, setSocketStatus] = useState<RealtimeStatus>("idle");
  const client = useMemo(() => new RealtimeClient(), []);

  useEffect(() => {
    client.setTokenProvider(() => sessionAccessToken());
    const offStatus = client.onStatus(setSocketStatus);
    const offMessage = client.onMessage((message: RealtimeServerMessage) => {
      if (message.type === "notification.created") {
        void queryClient.invalidateQueries({ queryKey: notificationKeys.all });
        void queryClient.invalidateQueries({ queryKey: notificationKeys.unread });
        const current = queryClient.getQueryData<{ count: number }>(notificationKeys.unread);
        if (current) {
          queryClient.setQueryData(notificationKeys.unread, {
            count: current.count + 1,
          });
        }
        return;
      }
      if (message.type === "realtime.resync") {
        // The socket dropped and the in-memory server hub missed everything
        // published in between, so live event pushes cannot be trusted for
        // that window. Refetch what normally arrives via push; polls and DMs
        // also poll on an interval, so this only shortens the gap.
        void queryClient.invalidateQueries({ queryKey: notificationKeys.all });
        void queryClient.invalidateQueries({ queryKey: dmKeys.all });
        void queryClient.invalidateQueries({ queryKey: ["polls"] });
        return;
      }
    });
    return () => {
      offStatus();
      offMessage();
      client.stop();
    };
  }, [client, queryClient]);

  useEffect(() => {
    if (authStatus === "authenticated") {
      client.start();
    } else {
      client.stop();
    }
  }, [authStatus, client]);

  const derivedStatus: RealtimeStatus =
    authStatus === "loading"
      ? "idle"
      : authStatus !== "authenticated"
        ? "disconnected"
        : socketStatus;

  const value = useMemo(
    () => ({
      status: derivedStatus,
      client: authStatus === "authenticated" ? client : null,
    }),
    [authStatus, client, derivedStatus],
  );

  return <RealtimeContext.Provider value={value}>{children}</RealtimeContext.Provider>;
}

export function useRealtime(): RealtimeContextValue {
  return useContext(RealtimeContext);
}
