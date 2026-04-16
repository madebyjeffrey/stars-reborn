use axum::{extract::State, Json};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Serialize;

use crate::{
    error::AppError,
    features::auth::refresh_sessions::model::{self, Entity as RefreshSessionEntity},
    jwt::{issue_access_token, TokenType},
    middleware::auth::AuthUser,
    AppState,
};

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
}

pub async fn refresh(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<RefreshResponse>, AppError> {
    // Verify that incoming token is a refresh token
    if auth_user.claims.typ != Some(TokenType::Refresh) {
        return Err(AppError::Auth("Invalid token type for refresh endpoint".to_string()));
    }

    let jti = auth_user
        .claims
        .jti
        .clone()
        .ok_or_else(|| AppError::Auth("Refresh token missing jti".to_string()))?;

    // Look up the refresh session
    let session = RefreshSessionEntity::find()
        .filter(model::Column::Jti.eq(&jti))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::Auth("Refresh session not found".to_string()))?;

    // Check if revoked
    if session.revoked_at.is_some() {
        return Err(AppError::Auth("Refresh token has been revoked".to_string()));
    }

    // Check if already replaced (replay detection)
    if session.replaced_by.is_some() {
        return Err(AppError::Auth(
            "Refresh token has been rotated and is no longer valid".to_string(),
        ));
    }

    // Check expiration
    let now = Utc::now().fixed_offset();
    if session.expires_at <= now {
        return Err(AppError::Auth("Refresh token has expired".to_string()));
    }

    // Issue new access token (using old jti to keep session tracking simple for now)
    let access_token = issue_access_token(&session.user_id.to_string(), &state.config.jwt_secret)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create access token: {}", e)))?;

    // Update last_used_at
    let mut active: model::ActiveModel = session.into();
    active.last_used_at = Set(Some(now));
    active.update(&state.db).await?;

    Ok(Json(RefreshResponse { access_token }))
}

pub async fn logout(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let jti = auth_user
        .claims
        .jti
        .clone()
        .ok_or_else(|| AppError::Auth("Token missing jti for logout".to_string()))?;

    // Revoke the session
    let session = RefreshSessionEntity::find()
        .filter(model::Column::Jti.eq(&jti))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

    let mut active: model::ActiveModel = session.into();
    active.revoked_at = Set(Some(Utc::now().fixed_offset()));
    active.update(&state.db).await?;

    Ok(Json(serde_json::json!({ "message": "Logged out successfully" })))
}


