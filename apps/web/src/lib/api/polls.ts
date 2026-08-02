import type {
  CreatePollDraft,
  HotPollItem,
  Poll,
  PollResults,
  UpdatePollRequest,
  VotePollRequest,
} from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";
import { optionalAuthHeaders } from "@/lib/api/forum";

export const pollKeys = {
  poll: (id: string) => ["polls", "poll", id] as const,
  topicPoll: (topicId: string) => ["polls", "topic", topicId] as const,
  results: (id: string) => ["polls", "results", id] as const,
  hot: ["polls", "hot"] as const,
};

export async function getPoll(pollId: string): Promise<Poll> {
  return apiRequest<Poll>(`/polls/${encodeURIComponent(pollId)}`, {
    headers: await optionalAuthHeaders(),
  });
}

export async function getTopicPoll(topicId: string): Promise<Poll> {
  return apiRequest<Poll>(`/topics/${encodeURIComponent(topicId)}/poll`, {
    headers: await optionalAuthHeaders(),
  });
}

export function createPoll(topicId: string, input: CreatePollDraft): Promise<Poll> {
  return apiRequest<Poll>(
    `/topics/${encodeURIComponent(topicId)}/poll`,
    { method: "POST", body: JSON.stringify(input) },
    true,
  );
}

export function votePoll(pollId: string, input: VotePollRequest): Promise<Poll> {
  return apiRequest<Poll>(
    `/polls/${encodeURIComponent(pollId)}/vote`,
    { method: "POST", body: JSON.stringify(input) },
    true,
  );
}

export function cancelPollVote(pollId: string, optionId?: string): Promise<Poll> {
  return apiRequest<Poll>(
    `/polls/${encodeURIComponent(pollId)}/vote`,
    {
      method: "DELETE",
      body: JSON.stringify(optionId ? { option_id: optionId } : {}),
    },
    true,
  );
}

export async function getPollResults(pollId: string): Promise<PollResults> {
  return apiRequest<PollResults>(`/polls/${encodeURIComponent(pollId)}/results`, {
    headers: await optionalAuthHeaders(),
  });
}

export function updatePoll(pollId: string, input: UpdatePollRequest): Promise<Poll> {
  return apiRequest<Poll>(
    `/polls/${encodeURIComponent(pollId)}`,
    { method: "PATCH", body: JSON.stringify(input) },
    true,
  );
}

export function closePoll(pollId: string): Promise<{ message: string }> {
  return apiRequest<{ message: string }>(
    `/polls/${encodeURIComponent(pollId)}/close`,
    { method: "POST" },
    true,
  );
}

export function deletePoll(pollId: string): Promise<{ message: string }> {
  return apiRequest<{ message: string }>(
    `/polls/${encodeURIComponent(pollId)}`,
    { method: "DELETE" },
    true,
  );
}

export async function listHotPolls(): Promise<HotPollItem[]> {
  return apiRequest<HotPollItem[]>("/polls/hot", {
    headers: await optionalAuthHeaders(),
  });
}
