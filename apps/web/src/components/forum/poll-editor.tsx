"use client";

import { Button, Input, Label, Textarea } from "@lumiforum/ui";
import { useFieldArray, useFormContext } from "react-hook-form";
import {
  ArrowDown,
  ArrowUp,
  BarChart3,
  GripVertical,
  Plus,
  Trash2,
} from "lucide-react";
import { useEffect, useState, type DragEvent } from "react";

import type { CreatePollDraft, Poll, UpdatePollRequest } from "@lumiforum/types";
import type { TopicEditorValues } from "@/lib/forum/schemas";
import { cn } from "@lumiforum/ui";

/**
 * Poll editor used by the topic editor.
 *
 * - create mode (no `existing`): builds a fresh poll draft.
 * - edit mode (`existing` set): loads the current poll; the author may update
 *   title / description / expiry / allow_cancel, add options, and remove
 *   options that have zero votes. Type toggles are locked once published.
 */
export function PollEditor({ existing }: { existing?: Poll }) {
  const { register, watch, setValue, formState } = useFormContext<TopicEditorValues>();
  const enabled = watch("poll.enabled");
  const multipleChoice = watch("poll.multiple_choice");
  const { fields, append, remove, swap } = useFieldArray<TopicEditorValues>({
    name: "poll.options",
  });
  const [dragIndex, setDragIndex] = useState<number | null>(null);

  const editMode = Boolean(existing);
  const locked = editMode && existing?.status === "closed";

  // Sync the form when the existing poll loads (edit mode).
  useEffect(() => {
    if (!existing) return;
    setValue("poll.enabled", true);
    setValue("poll.title", existing.title);
    setValue("poll.description", existing.description ?? "");
    setValue("poll.multiple_choice", existing.multiple_choice);
    setValue("poll.anonymous", existing.anonymous);
    setValue("poll.allow_cancel", existing.allow_cancel);
    setValue("poll.max_choices", existing.max_choices);
    setValue("poll.expires_at", existing.expires_at ? toLocalInput(existing.expires_at) : "");
    setValue(
      "poll.options",
      existing.options.map((option) => ({
        value: option.content,
        existing_id: option.id,
        vote_count: option.vote_count,
      })),
    );
  }, [existing, setValue]);

  const onDragStart = (index: number) => setDragIndex(index);
  const onDragOver = (event: DragEvent) => event.preventDefault();
  const onDrop = (index: number) => {
    if (dragIndex !== null && dragIndex !== index) swap(dragIndex, index);
    setDragIndex(null);
  };

  return (
    <section className="rounded-xl border border-border bg-surface/60 p-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <BarChart3 className="size-5 text-primary" aria-hidden="true" />
          <div>
            <h3 className="font-semibold">{editMode ? "编辑投票" : "帖子投票"}</h3>
            <p className="text-xs text-muted-foreground">
              {editMode
                ? "修改投票标题、描述、截止时间与选项"
                : "为帖子附加一个投票，读者可直接参与"}
            </p>
          </div>
        </div>
        <label className="inline-flex cursor-pointer items-center gap-2 text-sm">
          <input
            type="checkbox"
            className="size-4 accent-primary"
            disabled={editMode}
            {...register("poll.enabled")}
          />
          启用投票
        </label>
      </div>

      {editMode ? (
        <p className="mt-3 rounded-md bg-muted/60 px-3 py-2 text-xs text-muted-foreground">
          已有投票不可停用；要停止投票请在帖子页使用「结束投票」。
          {locked ? " 投票已结束，仅可查看。" : ""}
        </p>
      ) : null}

      {enabled ? (
        <div className={cn("mt-5 space-y-4", locked ? "pointer-events-none opacity-50" : "")}>
          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <Label htmlFor="poll-title">投票标题</Label>
              <Input
                id="poll-title"
                className="mt-1.5"
                placeholder="例如：周末去哪里？"
                aria-invalid={Boolean(formState.errors.poll?.title)}
                {...register("poll.title")}
              />
              <p className="mt-1 text-xs text-destructive">
                {formState.errors.poll?.title?.message}
              </p>
            </div>
            <div>
              <Label htmlFor="poll-expires">截止时间（可选）</Label>
              <Input
                id="poll-expires"
                type="datetime-local"
                className="mt-1.5"
                step={60}
                {...register("poll.expires_at")}
              />
            </div>
          </div>

          <div>
            <Label htmlFor="poll-description">投票描述（可选）</Label>
            <Textarea
              id="poll-description"
              className="mt-1.5 min-h-16"
              placeholder="补充投票说明"
              {...register("poll.description")}
            />
          </div>

          <div className="flex flex-wrap gap-6 text-sm">
            <label
              className={cn(
                "inline-flex cursor-pointer items-center gap-2",
                editMode ? "cursor-not-allowed opacity-60" : "",
              )}
              title={editMode ? "投票类型发布后不可修改" : undefined}
            >
              <input
                type="checkbox"
                className="size-4 accent-primary"
                disabled={editMode}
                {...register("poll.multiple_choice")}
              />
              多选
            </label>
            <label
              className={cn(
                "inline-flex cursor-pointer items-center gap-2",
                editMode ? "cursor-not-allowed opacity-60" : "",
              )}
              title={editMode ? "投票类型发布后不可修改" : undefined}
            >
              <input
                type="checkbox"
                className="size-4 accent-primary"
                disabled={editMode}
                {...register("poll.anonymous")}
              />
              匿名投票
            </label>
            <label className="inline-flex cursor-pointer items-center gap-2">
              <input
                type="checkbox"
                className="size-4 accent-primary"
                {...register("poll.allow_cancel")}
              />
              允许投票人取消投票
            </label>
            {multipleChoice ? (
              <label className="inline-flex items-center gap-2">
                最多可选
                <Input
                  type="number"
                  min={1}
                  max={20}
                  className="mt-0 h-8 w-20"
                  disabled={editMode}
                  aria-invalid={Boolean(formState.errors.poll?.max_choices)}
                  {...register("poll.max_choices", { valueAsNumber: true })}
                />
                项
              </label>
            ) : null}
          </div>
          {formState.errors.poll?.max_choices ? (
            <p className="text-xs text-destructive">
              {formState.errors.poll.max_choices.message}
            </p>
          ) : null}

          <div>
            <div className="mb-2 flex items-center justify-between">
              <Label>选项（{fields.length}）</Label>
              <Button
                type="button"
                size="sm"
                variant="outline"
                className="gap-1"
                disabled={fields.length >= 20}
                onClick={() => append({ value: "", existing_id: null, vote_count: 0 })}
              >
                <Plus className="size-3.5" aria-hidden="true" />
                添加选项
              </Button>
            </div>
            {editMode ? (
              <p className="mb-2 text-xs text-muted-foreground">
                新增选项将被追加到末尾；已有票数的选项不可删除（拖动排序仅在创建时可用）。
              </p>
            ) : null}
            <ul className="space-y-2">
              {fields.map((field, index) => {
                const votes = (field as { vote_count?: number }).vote_count ?? 0;
                const isExisting = Boolean((field as { existing_id?: string | null }).existing_id);
                const canRemove = !isExisting || votes === 0;
                return (
                  <li
                    key={field.id}
                    draggable={!editMode}
                    onDragStart={() => onDragStart(index)}
                    onDragOver={onDragOver}
                    onDrop={() => onDrop(index)}
                    onDragEnd={() => setDragIndex(null)}
                    className={cn(
                      "flex items-center gap-2 rounded-md border border-border bg-white px-2 py-1.5",
                      dragIndex === index ? "opacity-50" : "",
                    )}
                  >
                    {editMode ? (
                      <span className="w-4 shrink-0" aria-hidden="true" />
                    ) : (
                      <GripVertical
                        className="size-4 shrink-0 cursor-grab text-muted-foreground/50"
                        aria-hidden="true"
                      />
                    )}
                    <span className="w-6 shrink-0 text-center text-xs tabular-nums text-muted-foreground">
                      {index + 1}
                    </span>
                    <Input
                      className="h-9"
                      placeholder={`选项 ${index + 1}`}
                      aria-invalid={Boolean(formState.errors.poll?.options?.[index]?.value)}
                      {...register(`poll.options.${index}.value`)}
                    />
                    {isExisting ? (
                      <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
                        {votes} 票
                      </span>
                    ) : null}
                    {!editMode ? (
                      <Button
                        type="button"
                        size="sm"
                        variant="ghost"
                        className="size-8 shrink-0 px-0"
                        title="上移"
                        disabled={index === 0}
                        onClick={() => swap(index, index - 1)}
                      >
                        <ArrowUp className="size-4" aria-hidden="true" />
                      </Button>
                    ) : null}
                    {!editMode ? (
                      <Button
                        type="button"
                        size="sm"
                        variant="ghost"
                        className="size-8 shrink-0 px-0"
                        title="下移"
                        disabled={index === fields.length - 1}
                        onClick={() => swap(index, index + 1)}
                      >
                        <ArrowDown className="size-4" aria-hidden="true" />
                      </Button>
                    ) : null}
                    <Button
                      type="button"
                      size="sm"
                      variant="ghost"
                      className="size-8 shrink-0 px-0 text-destructive hover:text-destructive"
                      title={canRemove ? "删除选项" : "已有票数，无法删除"}
                      disabled={fields.length <= 2 || !canRemove}
                      onClick={() => remove(index)}
                    >
                      <Trash2 className="size-4" aria-hidden="true" />
                    </Button>
                  </li>
                );
              })}
            </ul>
            <p className="mt-2 text-xs text-destructive">
              {formState.errors.poll?.options?.message}
            </p>
          </div>
        </div>
      ) : null}
    </section>
  );
}

/** Convert editor values into the API draft (create mode). */
export function pollDraftFromValues(
  poll: TopicEditorValues["poll"],
): CreatePollDraft | undefined {
  if (!poll?.enabled) return undefined;
  const options = (poll.options ?? []).map((option) => option.value.trim()).filter(Boolean);
  if (options.length < 2) return undefined;
  return {
    title: poll.title.trim(),
    description: poll.description.trim() || undefined,
    multiple_choice: poll.multiple_choice,
    anonymous: poll.anonymous,
    allow_cancel: poll.allow_cancel,
    max_choices: poll.multiple_choice ? poll.max_choices : undefined,
    expires_at: poll.expires_at ? new Date(poll.expires_at).toISOString() : undefined,
    options,
  };
}

/** Convert editor values into the API patch for an existing poll (edit mode). */
export function pollUpdateFromValues(
  poll: TopicEditorValues["poll"],
  existing: Poll,
): UpdatePollRequest {
  const keepIds = new Set(
    (poll.options ?? [])
      .map((option) => (option as { existing_id?: string | null }).existing_id)
      .filter((id): id is string => Boolean(id)),
  );
  const optionsToAdd = (poll.options ?? [])
    .filter((option) => !(option as { existing_id?: string | null }).existing_id)
    .map((option) => option.value.trim())
    .filter(Boolean);
  const idsToRemove = existing.options
    .filter((option) => !keepIds.has(option.id))
    .map((option) => option.id);
  return {
    title: poll.title.trim(),
    description: poll.description.trim() || null,
    expires_at: poll.expires_at ? new Date(poll.expires_at).toISOString() : null,
    allow_cancel: poll.allow_cancel,
    options_to_add: optionsToAdd,
    option_ids_to_remove: idsToRemove,
  };
}

function toLocalInput(iso: string): string {
  const date = new Date(iso);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}
