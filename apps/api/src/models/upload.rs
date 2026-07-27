use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadCategory {
    Avatar,
    TopicImage,
    CommentImage,
    Attachment,
}

impl UploadCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Avatar => "avatar",
            Self::TopicImage => "topic_image",
            Self::CommentImage => "comment_image",
            Self::Attachment => "attachment",
        }
    }

    pub const fn max_bytes(self) -> usize {
        match self {
            Self::Avatar => 5 * 1024 * 1024,
            Self::TopicImage => 10 * 1024 * 1024,
            Self::CommentImage => 8 * 1024 * 1024,
            Self::Attachment => 20 * 1024 * 1024,
        }
    }

    pub const fn is_image(self) -> bool {
        !matches!(self, Self::Attachment)
    }
}

impl FromStr for UploadCategory {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "avatar" => Ok(Self::Avatar),
            "topic_image" => Ok(Self::TopicImage),
            "comment_image" => Ok(Self::CommentImage),
            "attachment" => Ok(Self::Attachment),
            _ => Err("unknown upload category"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UploadStatus {
    Pending,
    Ready,
    Deleting,
    Deleted,
    Failed,
}

impl UploadStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Deleting => "deleting",
            Self::Deleted => "deleted",
            Self::Failed => "failed",
        }
    }
}

impl FromStr for UploadStatus {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "ready" => Ok(Self::Ready),
            "deleting" => Ok(Self::Deleting),
            "deleted" => Ok(Self::Deleted),
            "failed" => Ok(Self::Failed),
            _ => Err("unknown upload status"),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct UploadResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub storage_provider: String,
    pub mime_type: String,
    pub file_size: i64,
    pub category: UploadCategory,
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct UploadListQuery {
    pub category: Option<UploadCategory>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}
