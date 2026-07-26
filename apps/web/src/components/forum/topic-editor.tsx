"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import type { CreateTopicRequest, TopicDetail, UpdateTopicRequest } from "@lumiforum/types";
import { Alert, Button, Input, Label, Select, Textarea } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CircleAlert, Eye, FilePenLine, PenLine, Send } from "lucide-react";
import type { Route } from "next";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { useForm, useWatch } from "react-hook-form";

import { MarkdownContent } from "@/components/forum/markdown-content";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { LoadingIndicator } from "@/components/loading-indicator";
import { errorMessage } from "@/lib/api/errors";
import { createTopic, forumKeys, listCategories, updateTopic } from "@/lib/api/forum";
import { topicEditorSchema, type TopicEditorValues } from "@/lib/forum/schemas";

type Props = { mode: "create"; topic?: never } | { mode: "edit"; topic: TopicDetail };

export function TopicEditor(props: Props) {
  const router = useRouter();
  const queryClient = useQueryClient();
  const [view, setView] = useState<"write" | "preview">("write");
  const categories = useQuery({ queryKey: forumKeys.categories, queryFn: listCategories });
  const form = useForm<TopicEditorValues>({
    resolver: zodResolver(topicEditorSchema),
    defaultValues: {
      categoryId: props.topic?.category.id ?? "",
      title: props.topic?.title ?? "",
      content: props.topic?.content ?? "",
      summary: props.topic?.summary ?? "",
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
        };
        return createTopic(input);
      }
      const input: UpdateTopicRequest = {
        category_id: values.categoryId,
        title: values.title,
        content: values.content,
        summary: values.summary || null,
      };
      return updateTopic(props.topic.id, input);
    },
    onSuccess: async (topic) => {
      queryClient.setQueryData(forumKeys.topic(topic.slug), topic);
      await queryClient.invalidateQueries({ queryKey: ["forum", "topics"] });
      await queryClient.invalidateQueries({ queryKey: forumKeys.categories });
      router.push(`/topics/${topic.slug}` as Route);
    },
    onError: (error) => form.setError("root", { message: errorMessage(error) }),
  });
  const content = useWatch({ control: form.control, name: "content" });

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

      <form onSubmit={form.handleSubmit((values) => mutation.mutate(values))}>
        {form.formState.errors.root?.message ? (
          <Alert className="mb-5">
            <CircleAlert className="size-4 shrink-0" aria-hidden="true" />
            {form.formState.errors.root.message}
          </Alert>
        ) : null}

        <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_260px]">
          <div className="min-w-0 space-y-5">
            <Field label="标题" error={form.formState.errors.title?.message} htmlFor="topic-title">
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
                  {...form.register("content")}
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
              <p className="mt-2 min-h-5 text-sm text-destructive">
                {form.formState.errors.content?.message}
              </p>
            </div>
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
                  <option key={category.id} value={category.id}>
                    {category.name}
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

            <Button type="submit" className="w-full gap-2" disabled={mutation.isPending}>
              {mutation.isPending ? (
                <LoadingIndicator />
              ) : (
                <Send className="size-4" aria-hidden="true" />
              )}
              {mutation.isPending ? "正在保存" : props.mode === "create" ? "发布帖子" : "保存修改"}
            </Button>
          </aside>
        </div>
      </form>
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
