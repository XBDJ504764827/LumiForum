import { z } from "zod";

export const commentEditorSchema = z.object({
  content: z.string().trim().min(1, "请输入评论内容").max(20_000, "评论不能超过 20000 个字符"),
});

export type CommentEditorValues = z.infer<typeof commentEditorSchema>;
