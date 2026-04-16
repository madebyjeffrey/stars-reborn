use axum::{extract::State, Json};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError,
    features::users::model::{self, Entity as UserEntity},
    AppState,
};

use super::super::super::users::handler::UserResponse;
use super::password::{hash_password, verify_password};

fn map_user_unique_constraint_error(err: sea_orm::DbErr) -> AppError {
    let msg = err.to_string().to_lowercase();

    if msg.contains("users_username_key") || (msg.contains("username") && msg.contains("unique")) {
        return AppError::Conflict("Username already taken".to_string());
    }

    if msg.contains("users_email_key") || (msg.contains("email") && msg.contains("unique")) {
        return AppError::Conflict("Email already in use".to_string());
    }

    AppError::Database(err)
}

fn create_jwt(user_id: &str, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
        iat: usize,
    }

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        exp: now + 86400 * 7, // 7 days
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let existing = UserEntity::find()
        .filter(model::Column::Username.eq(&req.username))
        .one(&state.db)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("Username already taken".to_string()));
    }

    if let Some(email) = req.email.as_deref() {
        let existing_email = UserEntity::find()
            .filter(model::Column::Email.eq(email))
            .one(&state.db)
            .await?;

        if existing_email.is_some() {
            return Err(AppError::Conflict("Email already in use".to_string()));
        }
    }

    if req.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters".to_string(),
        ));
    }

    let password_hash = hash_password(&req.password)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to hash password: {}", e)))?;

    let now = Utc::now().fixed_offset();
    let id = Uuid::new_v4();
    let user = model::ActiveModel {
        id: Set(id),
        username: Set(req.username),
        email: Set(req.email),
        password_hash: Set(Some(password_hash)),
        discord_id: Set(None),
        discord_username: Set(None),
        discord_avatar: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let created = user
        .insert(&state.db)
        .await
        .map_err(map_user_unique_constraint_error)?;

    let token = create_jwt(&created.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create JWT: {}", e)))?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse::from(created),
    }))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let user = UserEntity::find()
        .filter(model::Column::Username.eq(&req.username))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::Auth("Invalid username or password".to_string()))?;

    let password_hash = user
        .password_hash
        .as_ref()
        .ok_or_else(|| AppError::Auth("No password set for this account".to_string()))?;

    let valid = verify_password(&req.password, password_hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password verification failed: {}", e)))?;

    if !valid {
        return Err(AppError::Auth("Invalid username or password".to_string()));
    }

    let token = create_jwt(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create JWT: {}", e)))?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse::from(user),
    }))
}

pub async fn logout() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "message": "Logged out successfully" }))
}
