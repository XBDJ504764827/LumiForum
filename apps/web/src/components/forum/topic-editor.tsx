"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import type { CreateTopicRequest, TopicDetail, UpdateTopicRequest } from "@lumiforum/types";
import { Alert, Button, Input, Label, Select, Textarea } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CircleAlert, Eye, FilePenLine, PenLine, Send } from "lucide-react";
import type { Route } from "next";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useRef, useState } from "react";
import { FormProvider, useForm, useWatch } from "react-hook-form";

import { useAuth } from "@/components/auth/auth-provider";
import { MarkdownContent } from "@/components/forum/markdown-content";
import {
  PollEditor,
  pollDraftFromValues,
  pollUpdateFromValues,
} from "@/components/forum/poll-editor";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { LoadingIndicator } from "@/components/loading-indicator";
import { FileUpload } from "@/components/uploads/file-upload";
import { errorMessage } from "@/lib/api/errors";
import { createTopic, forumKeys, listCategories, updateTopic } from "@/lib/api/forum";
import { createPoll, getTopicPoll, pollKeys, updatePoll } from "@/lib/api/polls";
import { topicEditorSchema, type TopicEditorValues } from "@/lib/forum/schemas";

type Props = { mode: "create"; topic?: never } | { mode: "edit"; topic: TopicDetail };

export function TopicEditor(props: Props) {
  const router = useRouter();
  const { user } = useAuth();
  const isStaff = Boolean(
    user &&
    ["moderator", "senior_moderator", "administrator", "super_administrator"].includes(
      user.role.code,
    ),
  );
  const queryClient = useQueryClient();
  const contentRef = useRef<HTMLTextAreaElement | null>(null);
  const [view, setView] = useState<"write" | "preview">("write");
  const categories = useQuery({ queryKey: forumKeys.categories, queryFn: listCategories });
  // Edit mode: load the existing poll so the author can edit it too.
  const existingPoll = useQuery({
    queryKey: pollKeys.topicPoll(props.topic?.id ?? ""),
    queryFn: () => getTopicPoll(props.topic!.id),
    enabled: props.mode === "edit" && Boolean(props.topic?.has_poll),
    staleTime: 30_000,
    retry: false,
  });
  const form = useForm<TopicEditorValues>({
    resolver: zodResolver(topicEditorSchema),
    defaultValues: {
      categoryId: props.topic?.category.id ?? "",
      title: props.topic?.title ?? "",
      content: props.topic?.content ?? "",
      summary: props.topic?.summary ?? "",
      poll: {
        enabled: false,
        title: "",
        description: "",
        multiple_choice: false,
        anonymous: false,
        allow_cancel: true,
        max_choices: 2,
        options: [{ value: "" }, { value: "" }],
      },
    },
  });
  const mutation = useMutation({
    mutationFn: async (values: TopicEditorValues) => {
      if (props.mode === "create") {
        const input: CreateTopicRequest = {
          category_id: values.categoryId,
          title: values.title,
          content: values.content,
          summary: values.summary || undefined,
          anonymous: values.anonymous && canAnonymous ? true : undefined,
          poll: pollDraftFromValues(values.poll),
        };
        return createTopic(input);
      }
      const input: UpdateTopicRequest = {
        category_id: values.categoryId,
        title: values.title,
        content: values.content,
        summary: values.summary || null,
      };
      const topic = await updateTopic(props.topic.id, input);
      // Poll changes: patch the existing poll, or attach a new one when the
      // topic previously had none and the author enabled the poll editor.
      if (props.mode === "edit") {
        const existing = existingPoll.data;
        if (existing) {
          await updatePoll(existing.id, pollUpdateFromValues(values.poll, existing));
        } else if (values.poll.enabled) {
          const draft = pollDraftFromValues(values.poll);
          if (draft) await createPoll(topic.id, draft);
        }
      }
      return topic;
    },
    onSuccess: async (topic) => {
      queryClient.setQueryData(forumKeys.topic(topic.slug), topic);
      await queryClient.invalidateQueries({ queryKey: ["forum", "topics"] });
      await queryClient.invalidateQueries({ queryKey: forumKeys.categories });
      if (props.mode === "edit") {
        await queryClient.invalidateQueries({ queryKey: pollKeys.topicPoll(topic.id) });
        await queryClient.invalidateQueries({ queryKey: pollKeys.results(topic.id) });
      }
      if (topic.status === "pending_review") {
        // Content was flagged by auto-moderation and awaits staff review.
        router.push(`/topics/new?pending=1` as Route);
        return;
      }
      router.push(`/topics/${topic.slug}` as Route);
    },
    onError: (error) => form.setError("root", { message: errorMessage(error) }),
  });
  const content = useWatch({ control: form.control, name: "content" });
  const categoryId = useWatch({ control: form.control, name: "categoryId" });
  const contentField = form.register("content");
  const selectedCategory = (categories.data ?? []).find((category) => category.id === categoryId);
  const canAnonymous = Boolean(selectedCategory?.allow_anonymous);

  const insertImage = (url: string, originalFilename: string) => {
    const current = form.getValues("content");
    const cursor = contentRef.current?.selectionStart ?? current.length;
    const alt = originalFilename.replace(/[\[\]]/g, "");
    const markdown = `![${alt || "image"}](${url})`;
    const prefix = cursor > 0 && current[cursor - 1] !== "\n" ? "\n" : "";
    const suffix = cursor < current.length && current[cursor] !== "\n" ? "\n" : "";
    const insertion = `${prefix}${markdown}${suffix}`;
    form.setValue("content", `${current.slice(0, cursor)}${insertion}${current.slice(cursor)}`, {
      shouldDirty: true,
      shouldValidate: true,
    });
    requestAnimationFrame(() => {
      const nextCursor = cursor + insertion.length;
      contentRef.current?.focus();
      contentRef.current?.setSelectionRange(nextCursor, nextCursor);
    });
  };

  if (categories.isPending) return <QueryLoading label="正在加载编辑器" />;
  if (categories.isError) return <QueryError message="无法加载板块，请稍后重试" />;

  return (
    <main className="mx-auto max-w-5xl px-5 py-9 sm:px-8">
      <div className="mb-7 flex items-end justify-between gap-5 border-b border-border pb-6">
        <div>
          <p className="flex items-center gap-2 text-sm font-medium text-primary">
            {props.mode === "create" ? (
              <PenLine className="size-4" aria-hidden="true" />
            ) : (
              <FilePenLine className="size-4" aria-hidden="true" />
            )}
            Markdown 编辑器
          </p>
          <h1 className="mt-2 text-3xl font-semibold">
            {props.mode === "create" ? "发布帖子" : "编辑帖子"}
          </h1>
        </div>
        {props.topic ? (
          <Link
            href={`/topics/${props.topic.slug}`}
            className="text-sm text-muted-foreground hover:text-foreground"
          >
            取消编辑
          </Link>
        ) : null}
      </div>

      <FormProvider {...form}>
        <form onSubmit={form.handleSubmit((values) => mutation.mutate(values))}>
          <FieldErrorSummary form={form} />
          {form.formState.errors.root?.message ? (
            <Alert className="mb-5">
              <CircleAlert className="size-4 shrink-0" aria-hidden="true" />
              {form.formState.errors.root.message}
            </Alert>
          ) : null}

          <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_260px]">
            <div className="min-w-0 space-y-5">
              <Field
                label="标题"
                error={form.formState.errors.title?.message}
                htmlFor="topic-title"
              >
                <Input
                  id="topic-title"
                  autoFocus
                  aria-invalid={Boolean(form.formState.errors.title)}
                  {...form.register("title")}
                />
              </Field>

              <div>
                <div className="mb-2 flex items-center justify-between gap-4">
                  <Label htmlFor="topic-content">正文</Label>
                  <div className="inline-flex rounded-md border border-border p-0.5">
                    <ModeButton
                      active={view === "write"}
                      onClick={() => setView("write")}
                      icon={PenLine}
                    >
                      编写
                    </ModeButton>
                    <ModeButton
                      active={view === "preview"}
                      onClick={() => setView("preview")}
                      icon={Eye}
                    >
                      预览
                    </ModeButton>
                  </div>
                </div>
                {view === "write" ? (
                  <Textarea
                    id="topic-content"
                    className="min-h-[420px] font-mono"
                    placeholder="# 标题&#10;&#10;使用 Markdown 编写内容..."
                    aria-invalid={Boolean(form.formState.errors.content)}
                    {...contentField}
                    ref={(element) => {
                      contentRef.current = element;
                      contentField.ref(element);
                    }}
                  />
                ) : (
                  <div className="min-h-[420px] border border-border bg-white px-5 py-2">
                    {content ? (
                      <MarkdownContent content={content} />
                    ) : (
                      <p className="py-5 text-sm text-muted-foreground">暂无可预览内容</p>
                    )}
                  </div>
                )}
                {view === "write" ? (
                  <div className="mt-3">
                    <FileUpload
                      category="topic_image"
                      accept="image/jpeg,image/png,image/webp,image/gif"
                      maxBytes={10 * 1024 * 1024}
                      onUploaded={(upload) => insertImage(upload.url, upload.original_filename)}
                    />
                  </div>
                ) : null}
                <p className="mt-2 min-h-5 text-sm text-destructive">
                  {form.formState.errors.content?.message}
                </p>
              </div>

              {props.mode === "create" ? (
                <PollEditor />
              ) : existingPoll.isPending ? (
                <p className="rounded-xl border border-border bg-surface/60 p-5 text-sm text-muted-foreground">
                  正在加载投票数据…
                </p>
              ) : (
                <PollEditor existing={existingPoll.data} />
              )}
            </div>

            <aside className="space-y-5 border-t border-border pt-6 lg:border-l lg:border-t-0 lg:pl-6 lg:pt-0">
              <Field
                label="板块"
                error={form.formState.errors.categoryId?.message}
                htmlFor="topic-category"
              >
                <Select
                  id="topic-category"
                  aria-invalid={Boolean(form.formState.errors.categoryId)}
                  {...form.register("categoryId")}
                >
                  <option value="">选择板块</option>
                  {(categories.data ?? []).map((category) => (
                    <option
                      key={category.id}
                      value={category.id}
                      disabled={category.restricted_posting && !isStaff}
                    >
                      {category.name}
                      {category.restricted_posting ? "（仅管理员）" : ""}
                    </option>
                  ))}
                </Select>
              </Field>

              <Field
                label="摘要（可选）"
                error={form.formState.errors.summary?.message}
                htmlFor="topic-summary"
              >
                <Textarea id="topic-summary" className="min-h-28" {...form.register("summary")} />
              </Field>

              {props.mode === "create" && canAnonymous ? (
                <label className="flex cursor-pointer items-start gap-2.5 rounded-md border border-border bg-white px-3 py-3 text-sm">
                  <input
                    type="checkbox"
                    className="mt-0.5 size-4 accent-primary"
                    {...form.register("anonymous")}
                  />
                  <span>
                    <span className="block font-medium">匿名发布</span>
                    <span className="block text-xs text-muted-foreground">
                      你的用户名与头像不会公开展示（管理员仍可查看）
                    </span>
                  </span>
                </label>
              ) : null}

              <Button type="submit" className="w-full gap-2" disabled={mutation.isPending}>
                {mutation.isPending ? (
                  <LoadingIndicator />
                ) : (
                  <Send className="size-4" aria-hidden="true" />
                )}
                {mutation.isPending
                  ? "正在保存"
                  : props.mode === "create"
                    ? "发布帖子"
                    : "保存修改"}
              </Button>
            </aside>
          </div>
        </form>
      </FormProvider>
    </main>
  );
}

function Field({
  label,
  error,
  htmlFor,
  children,
}: {
  label: string;
  error?: string;
  htmlFor: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
      <p className="min-h-5 text-sm text-destructive">{error}</p>
    </div>
  );
}

function ModeButton({
  active,
  onClick,
  icon: Icon,
  children,
}: {
  active: boolean;
  onClick: () => void;
  icon: typeof Eye;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      className={`inline-flex h-7 items-center gap-1.5 rounded-sm px-2 text-xs font-medium ${
        active ? "bg-muted text-foreground" : "text-muted-foreground hover:text-foreground"
      }`}
      onClick={onClick}
    >
      <Icon className="size-3.5" aria-hidden="true" />
      {children}
    </button>
  );
}

/** Aggregated field errors — surfaced above the submit button so validation
 *  failures are always visible even when the failing field lives inside a
 *  collapsed section (e.g. the poll editor). */
function FieldErrorSummary({ form }: { form: ReturnType<typeof useForm<TopicEditorValues>> }) {
  const errors = form.formState.errors;
  const messages: string[] = [];
  const collect = (value: unknown, prefix = "") => {
    if (!value || typeof value !== "object") return;
    for (const [key, entry] of Object.entries(value as Record<string, unknown>)) {
      const path = prefix ? `${prefix}.${key}` : key;
      if (entry && typeof entry === "object" && "message" in (entry as object)) {
        const message = (entry as { message?: string }).message;
        if (message) messages.push(message);
      } else if (Array.isArray(entry)) {
        entry.forEach((item, index) => collect(item, `${path}.${index}`));
      } else {
        collect(entry, path);
      }
    }
  };
  collect(errors);
  if (messages.length === 0) return null;
  return (
    <Alert className="mb-5 border-destructive/40 bg-destructive/5">
      <CircleAlert className="size-4 shrink-0 text-destructive" aria-hidden="true" />
      <div>
        <p className="font-medium text-destructive">请先修正以下问题：</p>
        <ul className="mt-1 list-disc space-y-0.5 pl-5 text-sm text-destructive">
          {messages.map((message, index) => (
            <li key={index}>{message}</li>
          ))}
        </ul>
      </div>
    </Alert>
  );
}
