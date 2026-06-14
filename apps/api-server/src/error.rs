use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn internal(code: impl Into<String>, error: impl std::fmt::Display) -> Self {
        let code = code.into();
        tracing::error!(code = %code, error = %error, "internal API error");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            code,
            "internal server error",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(ErrorResponse {
                code: self.code,
                message: self.message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_error_hides_underlying_details() {
        let error = ApiError::internal("QUERY_FAILED", "sqlx connect timeout");

        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.code, "QUERY_FAILED");
        assert_eq!(error.message, "internal server error");
    }
}
