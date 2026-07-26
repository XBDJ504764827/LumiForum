import { z } from "zod";

export const topicEditorSchema = z.object({
  categoryId: z.string().uuid("请选择有效板块"),
  title: z.string().trim().min(3, "标题至少需要 3 个字符").max(200, "标题不能超过 200 个字符"),
  content: z.string().trim().min(1, "请输入帖子内容").max(100_000, "内容不能超过 100000 个字符"),
  summary: z.string().trim().max(500, "摘要不能超过 500 个字符"),
});

export type TopicEditorValues = z.infer<typeof topicEditorSchema>;
