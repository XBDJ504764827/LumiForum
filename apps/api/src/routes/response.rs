use axum::{extract::rejection::JsonRejection, Json};
use serde::Serialize;

use crate::error::{AppError, AppResult};

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

pub fn parse_json<T>(payload: Result<Json<T>, JsonRejection>) -> AppResult<T> {
    payload
        .map(|Json(value)| value)
        .map_err(|_| AppError::Validation("invalid JSON request body"))
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: &'static str,
}
