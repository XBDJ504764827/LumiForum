"use client";

import type { Poll } from "@lumiforum/types";
import { Badge, Button } from "@lumiforum/ui";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  BarChart3,
  CheckCircle2,
  Circle,
  Clock3,
  Lock,
  Trash2,
  Users,
  XCircle,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { useAuth } from "@/components/auth/auth-provider";
import { useRealtime } from "@/components/realtime/realtime-provider";
import { errorMessage } from "@/lib/api/errors";
import { cancelPollVote, closePoll, deletePoll, pollKeys, votePoll } from "@/lib/api/polls";
import { cn } from "@lumiforum/ui";

export function PollCard({ poll }: { poll: Poll }) {
  const { status: authStatus, user } = useAuth();
  const { client } = useRealtime();
  const queryClient = useQueryClient();
  const [selected, setSelected] = useState<string[]>(poll.my_votes);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const active = poll.status === "active" && !isExpired(poll.expires_at);
  const hasVoted = poll.my_votes.length > 0;
  const showResults = !active || hasVoted || !poll.can_vote;
  const selectedSet = useMemo(() => new Set(selected), [selected]);

  // Live updates: subscribe to this poll's realtime channel; invalidate on
  // any poll.updated event so the card always shows fresh numbers.
  useEffect(() => {
    if (!client) return;
    client.send("subscribe.poll", { poll_id: poll.id });
    const off = client.onMessage((message) => {
      if (message.type === "poll.updated") {
        const data = message.data as { poll_id?: string };
        if (data.poll_id === poll.id) {
          void queryClient.invalidateQueries({ queryKey: pollKeys.poll(poll.id) });
          void queryClient.invalidateQueries({ queryKey: pollKeys.results(poll.id) });
          void queryClient.invalidateQueries({ queryKey: pollKeys.topicPoll(poll.topic_id) });
        }
      }
    });
    return () => {
      off();
      client.send("unsubscribe.poll", { poll_id: poll.id });
    };
  }, [client, poll.id, poll.topic_id, queryClient]);

  const vote = useMutation({
    mutationFn: async () => {
      if (selected.length === 0) {
        throw new Error("请先选择选项");
      }
      return votePoll(poll.id, { option_ids: selected });
    },
    onMutate: () => setSubmitting(true),
    onSuccess: (updated) => {
      setError(null);
      setSelected(updated.my_votes);
      queryClient.setQueryData(pollKeys.poll(poll.id), updated);
      void queryClient.invalidateQueries({ queryKey: pollKeys.results(poll.id) });
      void queryClient.invalidateQueries({ queryKey: ["forum", "topics"] });
    },
    onError: (err) => setError(errorMessage(err)),
    onSettled: () => setSubmitting(false),
  });

  const cancel = useMutation({
    mutationFn: () => cancelPollVote(poll.id),
    onSuccess: (updated) => {
      setError(null);
      setSelected([]);
      queryClient.setQueryData(pollKeys.poll(poll.id), updated);
      void queryClient.invalidateQueries({ queryKey: pollKeys.results(poll.id) });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  const close = useMutation({
    mutationFn: () => closePoll(poll.id),
    onSuccess: (_, __) => {
      void queryClient.invalidateQueries({ queryKey: pollKeys.poll(poll.id) });
      void queryClient.invalidateQueries({ queryKey: pollKeys.results(poll.id) });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  const remove = useMutation({
    mutationFn: () => deletePoll(poll.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: pollKeys.topicPoll(poll.topic_id) });
      void queryClient.invalidateQueries({ queryKey: pollKeys.poll(poll.id) });
      void queryClient.invalidateQueries({ queryKey: ["forum", "topics"] });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  const toggle = (optionId: string) => {
    setError(null);
    setSelected((current) => {
      if (current.includes(optionId)) {
        return current.filter((id) => id !== optionId);
      }
      if (poll.multiple_choice && current.length >= poll.max_choices) {
        return current;
      }
      return poll.multiple_choice ? [...current, optionId] : [optionId];
    });
  };

  const totalVotes = poll.total_votes;
  const percentage = (count: number) =>
    totalVotes > 0 ? Math.round((count / totalVotes) * 1000) / 10 : 0;

  return (
    <section className="my-6 rounded-xl border border-border bg-surface/60 p-5" aria-label="投票">
      <header className="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div className="flex items-start gap-3">
          <BarChart3 className="mt-0.5 size-5 text-primary" aria-hidden="true" />
          <div>
            <h2 className="text-lg font-semibold leading-6">{poll.title}</h2>
            {poll.description ? (
              <p className="mt-1 text-sm text-muted-foreground">{poll.description}</p>
            ) : null}
            <div className="mt-2 flex flex-wrap gap-2">
              {!active ? (
                <Badge className="gap-1 text-muted-foreground">
                  <XCircle className="size-3" aria-hidden="true" />
                  已结束
                </Badge>
              ) : null}
              {poll.multiple_choice ? (
                <Badge className="text-muted-foreground">多选（最多 {poll.max_choices} 项）</Badge>
              ) : (
                <Badge className="text-muted-foreground">单选</Badge>
              )}
              {poll.anonymous ? (
                <Badge className="gap-1 text-muted-foreground">
                  <Lock className="size-3" aria-hidden="true" />
                  匿名
                </Badge>
              ) : null}
              {poll.expires_at ? (
                <Badge className="gap-1 text-muted-foreground">
                  <Clock3 className="size-3" aria-hidden="true" />
                  {active ? "截止 " : "已于 "}
                  {formatDateTime(poll.expires_at)}
                </Badge>
              ) : null}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Users className="size-3.5" aria-hidden="true" />
          {poll.participant_count} 人参与 · {totalVotes} 票
        </div>
      </header>

      <div className="space-y-2.5" role="group" aria-label="投票选项">
        {poll.options.map((option) => {
          const voted = poll.my_votes.includes(option.id);
          const percent = percentage(option.vote_count);
          const picked = selectedSet.has(option.id);
          return (
            <button
              key={option.id}
              type="button"
              disabled={!active || !poll.can_vote || submitting || cancel.isPending}
              onClick={() => toggle(option.id)}
              className={cn(
                "relative w-full overflow-hidden rounded-lg border px-4 py-3 text-left transition-colors",
                showResults
                  ? "cursor-default border-border"
                  : "cursor-pointer hover:border-primary/50",
                voted ? "border-primary/60 bg-primary/5" : "border-border bg-white",
                picked && !showResults ? "border-primary bg-primary/10" : "",
              )}
            >
              {showResults ? (
                <div
                  className="absolute inset-y-0 left-0 bg-primary/10 transition-all duration-700"
                  style={{ width: `${Math.min(100, percent)}%` }}
                  aria-hidden="true"
                />
              ) : null}
              <div className="relative flex items-center justify-between gap-3">
                <span className="flex min-w-0 items-center gap-2.5 text-sm">
                  {showResults ? (
                    voted ? (
                      <CheckCircle2 className="size-4 shrink-0 text-primary" aria-hidden="true" />
                    ) : (
                      <Circle
                        className="size-4 shrink-0 text-muted-foreground/40"
                        aria-hidden="true"
                      />
                    )
                  ) : picked ? (
                    <CheckCircle2 className="size-4 shrink-0 text-primary" aria-hidden="true" />
                  ) : (
                    <Circle
                      className="size-4 shrink-0 text-muted-foreground/40"
                      aria-hidden="true"
                    />
                  )}
                  <span className="truncate">{option.content}</span>
                </span>
                {showResults ? (
                  <span className="flex shrink-0 items-center gap-3 text-xs text-muted-foreground">
                    <span className="tabular-nums">{option.vote_count} 票</span>
                    <span className="w-12 text-right font-medium tabular-nums text-foreground">
                      {percent}%
                    </span>
                  </span>
                ) : null}
              </div>
            </button>
          );
        })}
      </div>

      {error ? <p className="mt-3 text-sm text-destructive">{error}</p> : null}

      <div className="mt-4 flex flex-wrap items-center gap-3">
        {!showResults ? (
          <Button
            type="button"
            size="sm"
            disabled={selected.length === 0 || submitting}
            onClick={() => vote.mutate()}
          >
            {poll.multiple_choice && selected.length > 0
              ? `提交（${selected.length}/${poll.max_choices}）`
              : "提交投票"}
          </Button>
        ) : hasVoted && active ? (
          poll.allow_cancel ? (
            <Button
              type="button"
              size="sm"
              variant="outline"
              disabled={cancel.isPending}
              onClick={() => cancel.mutate()}
            >
              取消我的投票
            </Button>
          ) : (
            <span className="text-xs text-muted-foreground">该投票不允许取消已投出的票</span>
          )
        ) : null}

        {poll.can_manage ? (
          <>
            {active ? (
              <Button
                type="button"
                size="sm"
                variant="outline"
                disabled={close.isPending}
                onClick={() => close.mutate()}
              >
                结束投票
              </Button>
            ) : null}
            {authStatus === "authenticated" &&
            user?.role.code !== "moderator" &&
            (user?.role.code === "administrator" || user?.role.code === "super_administrator") ? (
              <Button
                type="button"
                size="sm"
                variant="ghost"
                className="text-destructive hover:text-destructive"
                disabled={remove.isPending}
                onClick={() => {
                  if (window.confirm("确定删除该投票吗？此操作不可恢复。")) remove.mutate();
                }}
              >
                <Trash2 className="size-3.5" aria-hidden="true" />
                删除投票
              </Button>
            ) : null}
          </>
        ) : null}
      </div>
    </section>
  );
}

function isExpired(value: string | null): boolean {
  if (!value) return false;
  return new Date(value).getTime() <= Date.now();
}

function formatDateTime(value: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}
