/**
 * Local-storage drafts for the topic editor (create mode only). Auto-saved as
 * the user types so a refresh or navigation never loses a long post.
 */

const DRAFT_KEY_PREFIX = "lumiforum:draft:topic:";

export type TopicDraft = {
  categoryId: string;
  title: string;
  content: string;
  summary: string;
  anonymous: boolean;
  poll: {
    enabled: boolean;
    title: string;
    description: string;
    multiple_choice: boolean;
    anonymous: boolean;
    max_choices: number;
    options: Array<{ value: string }>;
  };
  savedAt: number;
};

export function draftKey(userId: string | undefined): string {
  // Drafts are keyed by user so different accounts never share content.
  return `${DRAFT_KEY_PREFIX}${userId ?? "anonymous"}`;
}

export function loadDraft(key: string): TopicDraft | null {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as TopicDraft;
    // Drop drafts older than 7 days.
    if (Date.now() - parsed.savedAt > 7 * 24 * 60 * 60 * 1000) {
      localStorage.removeItem(key);
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

export function saveDraft(key: string, draft: Omit<TopicDraft, "savedAt">): void {
  try {
    localStorage.setItem(key, JSON.stringify({ ...draft, savedAt: Date.now() }));
  } catch {
    // Storage full or unavailable — silently ignore.
  }
}

export function clearDraft(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    // ignore
  }
}

export function draftIsEmpty(draft: Omit<TopicDraft, "savedAt">): boolean {
  return (
    !draft.title.trim() && !draft.content.trim() && !draft.summary.trim() && !draft.poll.enabled
  );
}
