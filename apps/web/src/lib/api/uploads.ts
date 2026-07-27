import { getApiBaseUrl, joinUrl } from "@lumiforum/shared";
import type { ApiErrorBody, ApiResponse, Upload, UploadCategory, User } from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";
import { ApiClientError } from "@/lib/api/errors";
import { sessionAccessToken } from "@/lib/auth/session";

export interface UploadOptions {
  category: UploadCategory;
  avatar?: boolean;
  onProgress?: (percentage: number) => void;
  signal?: AbortSignal;
}

export async function uploadFile(file: File, options: UploadOptions): Promise<Upload | User> {
  return sendUpload(file, options, true);
}

export function deleteAvatar(): Promise<User> {
  return apiRequest<User>("/users/profile/avatar", { method: "DELETE" }, true);
}

async function sendUpload(
  file: File,
  options: UploadOptions,
  retryAfterRefresh: boolean,
): Promise<Upload | User> {
  const token = await sessionAccessToken(!retryAfterRefresh);
  const body = new FormData();
  body.append("file", file);
  if (!options.avatar) body.append("category", options.category);

  try {
    return await xhrUpload<Upload | User>(
      options.avatar ? "/users/profile/avatar" : "/uploads",
      body,
      token,
      options,
    );
  } catch (error) {
    if (error instanceof ApiClientError && error.status === 401 && retryAfterRefresh) {
      return sendUpload(file, options, false);
    }
    throw error;
  }
}

function xhrUpload<T>(
  path: string,
  body: FormData,
  token: string,
  options: UploadOptions,
): Promise<T> {
  return new Promise((resolve, reject) => {
    const request = new XMLHttpRequest();
    request.open("POST", joinUrl(getApiBaseUrl({ isServer: false }), path));
    request.setRequestHeader("authorization", `Bearer ${token}`);
    request.withCredentials = true;
    request.upload.onprogress = (event) => {
      if (event.lengthComputable) {
        options.onProgress?.(Math.round((event.loaded / event.total) * 100));
      }
    };
    request.onerror = () => reject(new ApiClientError(0, "network_error", "网络请求失败"));
    request.onabort = () => reject(new ApiClientError(0, "upload_aborted", "上传已取消"));
    request.onload = () => {
      let payload: ApiResponse<T> | ApiErrorBody | null = null;
      try {
        payload = request.responseText ? JSON.parse(request.responseText) : null;
      } catch {
        // Non-JSON failures use the generic message below.
      }
      if (request.status >= 200 && request.status < 300 && payload && "data" in payload) {
        resolve(payload.data);
        return;
      }
      const detail = payload && "error" in payload ? payload.error : null;
      reject(
        new ApiClientError(
          request.status,
          detail?.code ?? "request_failed",
          detail?.message ?? "上传失败，请稍后重试",
        ),
      );
    };
    const abort = () => request.abort();
    options.signal?.addEventListener("abort", abort, { once: true });
    request.onloadend = () => options.signal?.removeEventListener("abort", abort);
    request.send(body);
  });
}
