use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let AppError::Internal(err) = &self;
        tracing::error!(error = %err, "internal error");

        let status = StatusCode::INTERNAL_SERVER_ERROR;
        let code = "internal_error";
        let message = "internal server error".into();

        let body = Json(ErrorBody {
            error: ErrorDetail { code, message },
        });

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
