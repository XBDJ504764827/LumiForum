import type { ApiErrorBody } from "@lumiforum/types";

export class ApiClientError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = "ApiClientError";
  }
}

export async function apiError(response: Response): Promise<ApiClientError> {
  const body = await parseJson(response);
  const error = body as ApiErrorBody | null;
  return new ApiClientError(
    response.status,
    error?.error.code ?? "request_failed",
    error?.error.message ?? "请求失败，请稍后重试",
  );
}

export async function parseJson(response: Response): Promise<unknown> {
  const contentType = response.headers.get("content-type");
  if (!contentType?.includes("application/json")) {
    return null;
  }
  return response.json();
}

export function errorMessage(error: unknown): string {
  if (!(error instanceof ApiClientError)) {
    return "网络异常，请稍后重试";
  }
  return publicMessages[error.code] ?? error.message;
}

const publicMessages: Record<string, string> = {
  invalid_credentials: "用户名、邮箱或密码不正确",
  identity_conflict: "用户名或邮箱已被使用",
  account_unavailable: "账户当前不可用",
  invalid_refresh_token: "登录已过期，请重新登录",
  authentication_required: "请先登录",
  permission_denied: "当前账户没有此操作权限",
  not_found: "资源不存在",
  rate_limited: "操作过于频繁，请稍后再试",
  validation_error: "输入内容不符合要求",
  payload_too_large: "文件超过允许大小",
  unsupported_media_type: "不支持此文件类型",
  storage_unavailable: "文件存储暂时不可用，请稍后重试",
};
