import { z } from "zod";

export const topicEditorSchema = z.object({
  categoryId: z.string().uuid("请选择有效板块"),
  title: z.string().trim().min(3, "标题至少需要 3 个字符").max(200, "标题不能超过 200 个字符"),
  content: z.string().trim().min(1, "请输入帖子内容").max(100_000, "内容不能超过 100000 个字符"),
  summary: z.string().trim().max(500, "摘要不能超过 500 个字符"),
  anonymous: z.boolean().optional().default(false),
  // Poll fields are only validated when enabled; a disabled poll must never
  // block the topic form (empty draft values are the default state).
  poll: z
    .object({
      enabled: z.boolean(),
      title: z.string(),
      description: z.string(),
      multiple_choice: z.boolean(),
      anonymous: z.boolean(),
      // Default fills the value when the field was never registered (RHF
      // strips unregistered/default-missing fields on submit).
      allow_cancel: z.boolean().default(true),
      max_choices: z.coerce.number(),
      expires_at: z.string().optional(),
      options: z.array(
        z.object({
          value: z.string(),
          // Edit mode: id of the existing option (new options have none).
          existing_id: z.string().nullable().optional(),
          vote_count: z.number().optional(),
        }),
      ),
    })
    .superRefine((poll, ctx) => {
      if (!poll.enabled) return;
      const addIssue = (path: (string | number)[], message: string) => {
        ctx.addIssue({ code: z.ZodIssueCode.custom, path, message });
      };
      const title = poll.title.trim();
      if (!title) {
        addIssue(["title"], "投票标题不能为空");
      } else if (title.length > 200) {
        addIssue(["title"], "投票标题不能超过 200 个字符");
      }
      if (poll.description.trim().length > 2000) {
        addIssue(["description"], "投票描述不能超过 2000 个字符");
      }
      if (!Number.isInteger(poll.max_choices) || poll.max_choices < 1 || poll.max_choices > 20) {
        addIssue(["max_choices"], "最多可选数量必须在 1 到 20 之间");
      }
      if (poll.multiple_choice && poll.max_choices > poll.options.length) {
        addIssue(["max_choices"], "最多可选数量不能超过选项数量");
      }
      if (poll.options.length < 2) {
        addIssue(["options"], "投票至少需要 2 个选项");
      }
      if (poll.options.length > 20) {
        addIssue(["options"], "投票最多支持 20 个选项");
      }
      poll.options.forEach((option, index) => {
        if (!option.value.trim()) {
          addIssue(["options", index, "value"], "选项不能为空");
        } else if (option.value.trim().length > 500) {
          addIssue(["options", index, "value"], "选项不能超过 500 个字符");
        }
      });
    })
    .optional()
    .default({
      enabled: false,
      title: "",
      description: "",
      multiple_choice: false,
      anonymous: false,
      allow_cancel: true,
      max_choices: 2,
      expires_at: undefined,
      options: [{ value: "" }, { value: "" }],
    }),
});

export type TopicEditorValues = z.infer<typeof topicEditorSchema>;
export type PollEditorValues = TopicEditorValues["poll"];
