import { z } from "zod";

const username = z
  .string()
  .trim()
  .min(3, "用户名至少需要 3 个字符")
  .max(32, "用户名不能超过 32 个字符")
  .regex(/^[A-Za-z0-9][A-Za-z0-9_]*$/, "仅可使用字母、数字和下划线");

const password = z.string().min(8, "密码至少需要 8 个字符").max(128, "密码不能超过 128 个字符");

export const loginSchema = z.object({
  identifier: z.string().trim().min(1, "请输入用户名或邮箱"),
  password,
});

export const registerSchema = z
  .object({
    username,
    email: z.string().trim().email("请输入有效邮箱").max(254, "邮箱地址过长"),
    nickname: z.string().trim().max(64, "昵称不能超过 64 个字符").optional(),
    password,
    confirmPassword: z.string(),
  })
  .refine((values) => values.password === values.confirmPassword, {
    message: "两次输入的密码不一致",
    path: ["confirmPassword"],
  });

export const profileSchema = z.object({
  nickname: z.string().trim().max(64, "昵称不能超过 64 个字符"),
  avatar: z.string().trim().max(2048, "头像地址过长"),
});

export type LoginFormValues = z.infer<typeof loginSchema>;
export type RegisterFormValues = z.infer<typeof registerSchema>;
export type ProfileFormValues = z.infer<typeof profileSchema>;
