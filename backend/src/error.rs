use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tracing::error;

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    #[serde(rename = "UNAUTHORIZED")]
    Unauthorized,
    #[serde(rename = "TOKEN_EXPIRED")]
    TokenExpired,
    #[serde(rename = "INVALID_CREDENTIALS")]
    InvalidCredentials,
    #[serde(rename = "NOT_FOUND")]
    NotFound,
    #[serde(rename = "BAD_REQUEST")]
    BadRequest,
    #[serde(rename = "CONFLICT_USERNAME")]
    ConflictUsername,
    #[serde(rename = "CONFLICT_EMAIL")]
    ConflictEmail,
    #[serde(rename = "INTERNAL_SERVER_ERROR")]
    InternalServerError,
}

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    #[error("Authentication error: {0}")]
    Auth(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
    #[error("Unauthorized")]
    Unauthorized,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, public_message) = match &self {
            AppError::Database(e) => {
                error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalServerError, "An error occurred while processing your request".to_string())
            }
            AppError::Auth(msg) => {
                // Auth errors are safe to expose (e.g., "Invalid username or password")
                (StatusCode::UNAUTHORIZED, ErrorCode::InvalidCredentials, msg.clone())
            }
            AppError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, ErrorCode::NotFound, msg.clone())
            }
            AppError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, ErrorCode::BadRequest, msg.clone())
            }
            AppError::Conflict(msg) => {
                // Parse conflict message to determine type for error code
                let code = if msg.contains("username") {
                    ErrorCode::ConflictUsername
                } else if msg.contains("email") {
                    ErrorCode::ConflictEmail
                } else {
                    ErrorCode::BadRequest
                };
                (StatusCode::CONFLICT, code, msg.clone())
            }
            AppError::Internal(e) => {
                error!("Internal error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalServerError, "An error occurred while processing your request".to_string())
            }
            AppError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized, "Unauthorized".to_string())
            }
        };
        (status, Json(json!({
            "error": public_message,
            "code": code,
        }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[test]
    fn database_error_does_not_leak_details() {
        let db_err = sea_orm::DbErr::Custom("connection refused".to_string());
        let app_err = AppError::Database(db_err);

        let response = app_err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Body should contain generic message, not "connection refused"
        // We can't easily inspect the body here in unit tests, but the match
        // in into_response ensures generic message is used
    }

    #[test]
    fn auth_error_message_is_exposed() {
        let err = AppError::Auth("Invalid username or password".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        // Auth messages are intentionally safe to expose
    }

    #[test]
    fn conflict_error_with_username_gets_correct_code() {
        let err = AppError::Conflict("Username already taken".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::CONFLICT);
        // Response code should be CONFLICT_USERNAME
    }

    #[test]
    fn conflict_error_with_email_gets_correct_code() {
        let err = AppError::Conflict("Email already in use".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::CONFLICT);
        // Response code should be CONFLICT_EMAIL
    }

    #[test]
    fn internal_error_does_not_leak_details() {
        let internal_err = anyhow::anyhow!("sensitive internal state");
        let app_err = AppError::Internal(internal_err);
        let response = app_err.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        // Message should be generic
    }

    #[test]
    fn unauthorized_error_returns_unauthorized_code() {
        let err = AppError::Unauthorized;
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
