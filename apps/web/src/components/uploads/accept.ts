/**
 * Accepted MIME types for the upload pickers. These mirror the server-side
 * allowlist in apps/api/src/services/upload.rs — the server is authoritative
 * and always re-verifies content by magic bytes.
 */

/** Processed image formats (inline rendering). */
export const IMAGE_ACCEPT = "image/jpeg,image/png,image/webp,image/gif";

/** Everything else: documents, archives, audio and video (forced download). */
export const ATTACHMENT_ACCEPT = [
  // Documents
  "application/pdf",
  "text/plain",
  "application/msword",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "application/vnd.ms-excel",
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  "application/vnd.ms-powerpoint",
  "application/vnd.openxmlformats-officedocument.presentationml.presentation",
  "application/vnd.oasis.opendocument.text",
  "application/vnd.oasis.opendocument.spreadsheet",
  "application/vnd.oasis.opendocument.presentation",
  "application/rtf",
  // Archives
  "application/zip",
  "application/x-7z-compressed",
  "application/vnd.rar",
  "application/x-tar",
  "application/gzip",
  "application/x-bzip2",
  "application/x-xz",
  "application/zstd",
  // Audio
  "audio/mpeg",
  "audio/m4a",
  "audio/ogg",
  "audio/opus",
  "audio/x-flac",
  "audio/x-wav",
  "audio/aac",
  "audio/midi",
  // Video
  "video/mp4",
  "video/webm",
  "video/x-matroska",
  "video/quicktime",
  "video/x-msvideo",
  "video/x-ms-wmv",
  "video/mpeg",
  "video/x-m4v",
].join(",");

export const ATTACHMENT_HINT = "支持 PDF、Office 文档、压缩包、音视频等常见格式";
