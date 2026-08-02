import { z } from "zod";

export const topicEditorSchema = z.object({
  categoryId: z.string().uuid("请选择有效板块"),
  title: z.string().trim().min(3, "标题至少需要 3 个字符").max(200, "标题不能超过 200 个字符"),
  content: z.string().trim().min(1, "请输入帖子内容").max(100_000, "内容不能超过 100000 个字符"),
  summary: z.string().trim().max(500, "摘要不能超过 500 个字符"),
  poll: z
    .object({
      enabled: z.boolean(),
      title: z.string().trim().min(1, "投票标题不能为空").max(200, "投票标题不能超过 200 个字符"),
      description: z.string().trim().max(2000, "投票描述不能超过 2000 个字符"),
      multiple_choice: z.boolean(),
      anonymous: z.boolean(),
      allow_cancel: z.boolean(),
      max_choices: z.coerce.number().int().min(1, "至少可选 1 项").max(20, "最多可选 20 项"),
      expires_at: z.string().optional(),
      options: z
        .array(
          z.object({
            value: z.string().trim().min(1, "选项不能为空").max(500, "选项不能超过 500 个字符"),
            // Edit mode: id of the existing option (new options have none).
            existing_id: z.string().nullable().optional(),
            vote_count: z.number().optional(),
          }),
        )
        .min(2, "投票至少需要 2 个选项")
        .max(20, "投票最多支持 20 个选项"),
    })
    .superRefine((poll, ctx) => {
      if (!poll.enabled) return;
      if (poll.multiple_choice && poll.max_choices > poll.options.length) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ["max_choices"],
          message: "最多可选数量不能超过选项数量",
        });
      }
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
      options: [{ value: "" }, { value: "" }],
    }),
});

export type TopicEditorValues = z.infer<typeof topicEditorSchema>;
export type PollEditorValues = TopicEditorValues["poll"];
