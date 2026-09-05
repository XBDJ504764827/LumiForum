"use client";

import type { Route } from "next";
import type { ConversationSummary, DmMessage } from "@lumiforum/types";
import { Avatar, AvatarFallback, AvatarImage, Button, Textarea } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, Send } from "lucide-react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useEffect, useRef, useState } from "react";

import { useAuth } from "@/components/auth/auth-provider";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import {
  dmKeys,
  getConversation,
  listConversations,
  listMessages,
  markConversationRead,
  sendMessage,
} from "@/lib/api/dm";
import { getUserPresence } from "@/lib/api/presence";
import { errorMessage } from "@/lib/api/errors";

/**
 * Two-pane messenger: conversation list on the left, active thread on the
 * right. On narrow screens only one pane shows at a time.
 */
export function MessagesView() {
  const { user } = useAuth();
  const queryClient = useQueryClient();
  const searchParams = useSearchParams();
  // A "发私信" entry elsewhere deep-links here as /messages?c=<id>. The id is
  // used as the initial state so no effect/setState dance is needed.
  const [activeId, setActiveId] = useState<string | null>(searchParams.get("c"));
  const [mobileThreadOpen, setMobileThreadOpen] = useState(Boolean(searchParams.get("c")));

  const conversations = useQuery({
    queryKey: dmKeys.conversations,
    queryFn: listConversations,
    refetchInterval: 30_000,
  });

  const items = conversations.data?.items ?? [];

  const openConversation = (conversationId: string) => {
    setActiveId(conversationId);
    setMobileThreadOpen(true);
  };

  const onThreadRead = () => {
    void queryClient.invalidateQueries({ queryKey: dmKeys.conversations });
    void queryClient.invalidateQueries({ queryKey: dmKeys.unread });
  };

  return (
    <main className="mx-auto max-w-6xl px-5 py-9 sm:px-8">
      <div className="mb-8 border-b border-border pb-6">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Link href="/" className="hover:text-foreground">
            首页
          </Link>
          <span>/</span>
          <span>私信</span>
        </div>
        <h1 className="mt-3 text-3xl font-semibold">私信</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          与其他玩家一对一交流。私信不会公开，请遵守社区规范。
        </p>
      </div>

      {conversations.isPending ? (
        <QueryLoading label="正在加载会话" />
      ) : conversations.isError ? (
        <QueryError message={errorMessage(conversations.error)} />
      ) : (
        <div className="grid min-h-[60vh] overflow-hidden rounded-lg border border-border lg:grid-cols-[280px_minmax(0,1fr)]">
          <ul
            className={`divide-y divide-border border-border lg:border-r ${
              mobileThreadOpen ? "hidden lg:block" : "block"
            }`}
          >
            {items.length === 0 ? (
              <li className="px-4 py-14 text-center text-sm text-muted-foreground">
                暂无会话。在用户主页点击「发私信」开始对话。
              </li>
            ) : (
              items.map((conversation) => (
                <li key={conversation.id}>
                  <ConversationRow
                    conversation={conversation}
                    active={conversation.id === activeId}
                    onClick={() => openConversation(conversation.id)}
                  />
                </li>
              ))
            )}
          </ul>

          <div className={`${mobileThreadOpen ? "block" : "hidden lg:block"} min-w-0`}>
            {activeId ? (
              <MessageThread
                conversationId={activeId}
                viewerId={user?.id ?? ""}
                onBack={() => setMobileThreadOpen(false)}
                onRead={onThreadRead}
              />
            ) : (
              <div className="flex h-full items-center justify-center p-8 text-sm text-muted-foreground">
                选择一个会话开始阅读
              </div>
            )}
          </div>
        </div>
      )}
    </main>
  );
}

function ConversationRow(props: {
  conversation: ConversationSummary;
  active: boolean;
  onClick: () => void;
}) {
  const { conversation, active, onClick } = props;
  const other = conversation.other_user;
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex w-full items-center gap-3 px-4 py-3 text-left hover:bg-muted/40 ${
        active ? "bg-muted/60" : ""
      }`}
    >
      <Avatar className="size-10 shrink-0 border border-border">
        {other.avatar ? <AvatarImage src={other.avatar} alt="" /> : null}
        <AvatarFallback>{(other.nickname || other.username).slice(0, 2)}</AvatarFallback>
      </Avatar>
      <div className="min-w-0 flex-1">
        <div className="flex items-center justify-between gap-2">
          <p className="truncate font-medium">{other.nickname || other.username}</p>
          <span className="shrink-0 text-xs text-muted-foreground">
            {formatShortDate(conversation.last_message_at)}
          </span>
        </div>
        <div className="mt-0.5 flex items-center justify-between gap-2">
          <p className="truncate text-sm text-muted-foreground">
            {conversation.last_message_preview || "（暂无消息）"}
          </p>
          {conversation.unread_count > 0 ? (
            <span className="inline-flex min-w-5 items-center justify-center rounded-full bg-destructive px-1.5 text-[10px] font-semibold leading-4 text-white">
              {conversation.unread_count > 99 ? "99+" : conversation.unread_count}
            </span>
          ) : null}
        </div>
      </div>
    </button>
  );
}

function MessageThread(props: {
  conversationId: string;
  viewerId: string;
  onBack: () => void;
  onRead: () => void;
}) {
  const { conversationId, viewerId, onBack, onRead } = props;
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState("");
  const bottomRef = useRef<HTMLDivElement | null>(null);
  const knownIdsRef = useRef<Set<string>>(new Set());
  const firstLoadRef = useRef(true);

  const header = useQuery({
    queryKey: [...dmKeys.conversations, "header", conversationId],
    queryFn: () => getConversation(conversationId),
    staleTime: 60_000,
  });

  const messages = useQuery({
    queryKey: dmKeys.messages(conversationId),
    queryFn: () => listMessages(conversationId),
    // Newest-first page; rendered reversed. Poll for new messages.
    refetchInterval: 10_000,
  });

  // Mark read whenever the thread is open and there are unread messages.
  const markRead = useMutation({
    mutationFn: () => markConversationRead(conversationId),
    onSuccess: onRead,
  });
  const unread = header.data?.unread_count ?? 0;
  useEffect(() => {
    if (unread > 0) {
      markRead.mutate();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [unread, conversationId]);

  const timeline = [...(messages.data?.items ?? [])].reverse();

  // Keep the newest message in view: jump on first load, scroll smoothly for
  // messages that arrive afterwards.
  useEffect(() => {
    if (messages.data) {
      const incoming = messages.data.items;
      const isNewMessage =
        !firstLoadRef.current &&
        incoming.length > 0 &&
        !knownIdsRef.current.has(incoming[0]?.id ?? "");
      incoming.forEach((message) => knownIdsRef.current.add(message.id));
      if (firstLoadRef.current) {
        firstLoadRef.current = false;
        bottomRef.current?.scrollIntoView();
      } else if (isNewMessage) {
        bottomRef.current?.scrollIntoView({ behavior: "smooth" });
      }
    }
  }, [messages.data]);

  const [sendError, setSendError] = useState<string | null>(null);
  const sendPendingRef = useRef(false);
  const doSend = async () => {
    const content = draft.trim();
    if (!content || sendPendingRef.current) return;
    sendPendingRef.current = true;
    setSendError(null);
    setDraft("");
    try {
      await sendMessage(conversationId, { content });
    } catch (error) {
      setDraft(content); // put the text back so nothing is lost
      setSendError(errorMessage(error));
    } finally {
      sendPendingRef.current = false;
    }
    await queryClient.invalidateQueries({ queryKey: dmKeys.messages(conversationId) });
    await queryClient.invalidateQueries({ queryKey: dmKeys.conversations });
  };

  return (
    <div className="flex h-full min-h-[60vh] flex-col">
      <div className="flex items-center gap-3 border-b border-border px-4 py-3">
        <button
          type="button"
          onClick={onBack}
          className="inline-flex h-8 w-8 items-center justify-center rounded-md hover:bg-muted lg:hidden"
          aria-label="返回会话列表"
        >
          <ArrowLeft className="size-4" aria-hidden="true" />
        </button>
        {header.data ? (
          <>
            <Avatar className="size-9 border border-border">
              {header.data.other_user.avatar ? (
                <AvatarImage src={header.data.other_user.avatar} alt="" />
              ) : null}
              <AvatarFallback>
                {(header.data.other_user.nickname || header.data.other_user.username).slice(0, 2)}
              </AvatarFallback>
            </Avatar>
            <div className="min-w-0">
              <Link
                href={`/users/${header.data.other_user.id}` as Route}
                className="block truncate font-medium hover:text-primary hover:underline"
              >
                {header.data.other_user.nickname || header.data.other_user.username}
              </Link>
              <PresenceLine userId={header.data.other_user.id} />
            </div>
          </>
        ) : (
          <span className="h-9 w-40 animate-pulse rounded-md bg-muted" aria-hidden="true" />
        )}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4">
        {messages.isPending ? (
          <QueryLoading label="正在加载消息" />
        ) : messages.isError ? (
          <QueryError message={errorMessage(messages.error)} />
        ) : timeline.length === 0 ? (
          <p className="py-14 text-center text-sm text-muted-foreground">
            还没有消息，发送第一条吧。
          </p>
        ) : (
          <ul className="space-y-3">
            {timeline.map((message) => (
              <li key={message.id}>
                <MessageBubble message={message} mine={message.sender_id === viewerId} />
              </li>
            ))}
          </ul>
        )}
        <div ref={bottomRef} />
      </div>

      <form
        onSubmit={(event) => {
          event.preventDefault();
          void doSend();
        }}
        className="border-t border-border p-3"
      >
        <div className="flex items-end gap-2">
          <Textarea
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            placeholder="输入消息…（Enter 发送，Shift+Enter 换行）"
            className="min-h-11 flex-1"
            rows={2}
            maxLength={2000}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                void doSend();
              }
            }}
          />
          <Button
            type="submit"
            size="sm"
            className="size-11 shrink-0"
            disabled={!draft.trim()}
            aria-label="发送"
          >
            <Send className="size-4" aria-hidden="true" />
          </Button>
        </div>
        {sendError ? <p className="mt-2 text-sm text-destructive">{sendError}</p> : null}
      </form>
    </div>
  );
}

/**
 * Shows the peer's online state under their name in a thread. Messages are
 * durable: when the peer is offline the composer keeps working and the note
 * says the reply arrives once they are back.
 */
function PresenceLine({ userId }: { userId: string }) {
  const presence = useQuery({
    queryKey: ["presence", userId],
    queryFn: () => getUserPresence(userId),
    refetchInterval: 30_000,
    staleTime: 15_000,
  });
  if (presence.isPending) {
    return <p className="text-xs text-muted-foreground">…</p>;
  }
  if (presence.data?.online) {
    return (
      <p className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
        <span className="size-1.5 rounded-full bg-emerald-500" aria-hidden="true" />
        在线
      </p>
    );
  }
  return (
    <p className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
      <span className="size-1.5 rounded-full bg-muted-foreground/40" aria-hidden="true" />
      离线 · 消息会在对方上线后送达
    </p>
  );
}

function MessageBubble({ message, mine }: { message: DmMessage; mine: boolean }) {
  return (
    <div className={`flex ${mine ? "justify-end" : "justify-start"}`}>
      {message.is_deleted ? (
        <p className="rounded-md border border-dashed border-border px-3 py-1.5 text-sm text-muted-foreground">
          消息已删除
        </p>
      ) : (
        <div
          className={`max-w-[75%] rounded-lg px-3 py-2 text-sm leading-6 ${
            mine ? "bg-primary text-primary-foreground" : "border border-border bg-surface"
          }`}
        >
          <p className="whitespace-pre-wrap break-words">{message.content}</p>
          <p
            className={`mt-1 text-[10px] ${mine ? "text-primary-foreground/70" : "text-muted-foreground"}`}
          >
            {formatTime(message.created_at)}
          </p>
        </div>
      )}
    </div>
  );
}

function formatShortDate(value: string): string {
  const date = new Date(value);
  const now = new Date();
  const sameDay = date.toDateString() === now.toDateString();
  if (sameDay) {
    return formatTime(value);
  }
  return new Intl.DateTimeFormat("zh-CN", { month: "numeric", day: "numeric" }).format(date);
}

function formatTime(value: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    timeStyle: "short",
  }).format(new Date(value));
}
