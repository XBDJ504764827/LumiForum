import type {
  Notification,
  NotificationListParams,
  Paginated,
  UnreadCount,
} from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";

export const notificationKeys = {
  all: ["notifications"] as const,
  list: (params: NotificationListParams) => ["notifications", "list", params] as const,
  unread: ["notifications", "unread"] as const,
};

export function listNotifications(
  params: NotificationListParams = {},
): Promise<Paginated<Notification>> {
  const query = new URLSearchParams();
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  if (typeof params.is_read === "boolean") query.set("is_read", String(params.is_read));
  if (params.type) query.set("type", params.type);
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<Notification>>(`/notifications${suffix}`, undefined, true);
}

export function getUnreadCount(): Promise<UnreadCount> {
  return apiRequest<UnreadCount>("/notifications/unread-count", undefined, true);
}

export async function markNotificationRead(notificationId: string): Promise<void> {
  await apiRequest<{ message: string }>(
    `/notifications/${encodeURIComponent(notificationId)}/read`,
    { method: "PATCH" },
    true,
  );
}

export async function markAllNotificationsRead(): Promise<void> {
  await apiRequest<{ message: string }>("/notifications/read-all", { method: "POST" }, true);
}

export function notificationHref(notification: Notification): string {
  const meta = notification.metadata;
  if (typeof meta.href === "string" && meta.href.startsWith("/")) {
    return meta.href;
  }
  if (notification.target_type === "topic" && typeof meta.topic_slug === "string") {
    return `/topics/${meta.topic_slug}`;
  }
  if (notification.target_type === "comment" && typeof meta.topic_slug === "string") {
    const commentId =
      typeof meta.comment_id === "string" ? meta.comment_id : notification.target_id;
    return commentId
      ? `/topics/${meta.topic_slug}#comment-${commentId}`
      : `/topics/${meta.topic_slug}`;
  }
  if (notification.target_type === "user" && notification.target_id) {
    return `/users/${notification.target_id}/followers`;
  }
  return "/notifications";
}
