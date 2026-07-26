use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection, QueryRejection},
        Path, Query,
    },
    Json,
};
use serde::{de::DeserializeOwned, Serialize};

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

pub fn parse_path<T>(payload: Result<Path<T>, PathRejection>) -> AppResult<T>
where
    T: DeserializeOwned,
{
    payload
        .map(|Path(value)| value)
        .map_err(|_| AppError::Validation("invalid path parameter"))
}

pub fn parse_query<T>(payload: Result<Query<T>, QueryRejection>) -> AppResult<T>
where
    T: DeserializeOwned,
{
    payload
        .map(|Query(value)| value)
        .map_err(|_| AppError::Validation("invalid query parameters"))
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: &'static str,
}
