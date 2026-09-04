import type {
  ConversationDetail,
  ConversationSummary,
  DmMessage,
  Paginated,
  SendMessageRequest,
} from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";

export const dmKeys = {
  all: ["dm"] as const,
  conversations: ["dm", "conversations"] as const,
  unread: ["dm", "unread"] as const,
  messages: (conversationId: string) => ["dm", "messages", conversationId] as const,
};

export function listConversations(): Promise<Paginated<ConversationSummary>> {
  return apiRequest<Paginated<ConversationSummary>>(
    "/messages?page=1&page_size=50",
    undefined,
    true,
  );
}

export function getDmUnreadCount(): Promise<{ count: number }> {
  return apiRequest<{ count: number }>("/messages/unread-count", undefined, true);
}

export function startConversation(username: string): Promise<ConversationDetail> {
  return apiRequest<ConversationDetail>(
    "/messages/conversations",
    {
      method: "POST",
      body: JSON.stringify({ username } satisfies { username: string }),
    },
    true,
  );
}

export function getConversation(conversationId: string): Promise<ConversationDetail> {
  return apiRequest<ConversationDetail>(
    `/messages/conversations/${encodeURIComponent(conversationId)}`,
    undefined,
    true,
  );
}

export function listMessages(
  conversationId: string,
  page = 1,
  pageSize = 30,
): Promise<Paginated<DmMessage>> {
  const query = new URLSearchParams({
    page: String(page),
    page_size: String(pageSize),
  });
  return apiRequest<Paginated<DmMessage>>(
    `/messages/conversations/${encodeURIComponent(conversationId)}/messages?${query}`,
    undefined,
    true,
  );
}

export function sendMessage(conversationId: string, input: SendMessageRequest): Promise<DmMessage> {
  return apiRequest<DmMessage>(
    `/messages/conversations/${encodeURIComponent(conversationId)}/messages`,
    { method: "POST", body: JSON.stringify(input) },
    true,
  );
}

export async function markConversationRead(conversationId: string): Promise<void> {
  await apiRequest<{ message: string }>(
    `/messages/conversations/${encodeURIComponent(conversationId)}/read`,
    { method: "POST" },
    true,
  );
}

export async function deleteMessage(messageId: string): Promise<void> {
  // Soft delete shares the conversation route namespace; implemented as a
  // dedicated endpoint in the API.
  await apiRequest<{ message: string }>(
    `/messages/dm/${encodeURIComponent(messageId)}`,
    { method: "DELETE" },
    true,
  );
}
