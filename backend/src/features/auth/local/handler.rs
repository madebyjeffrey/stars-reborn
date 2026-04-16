use axum::{extract::State, Json};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar, SameSite};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use time;
use uuid::Uuid;

use crate::{
    error::AppError,
    features::users::{dto::UserResponse, model::{self, Entity as UserEntity}},
    features::auth::refresh_sessions::model::{self as refresh_model},
    jwt::{issue_access_token, Claims},
    AppState,
};

use super::password::{hash_password, verify_password};

async fn create_refresh_session(
    db: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<Claims, AppError> {
    let jti = Uuid::new_v4().to_string();
    let expires_at = now + chrono::Duration::days(30);
    let issued_at = now.timestamp() as usize;

    let session = refresh_model::ActiveModel {
        jti: Set(jti.clone()),
        user_id: Set(user_id),
        expires_at: Set(expires_at),
        revoked_at: Set(None),
        replaced_by: Set(None),
        created_at: Set(now),
        last_used_at: Set(None),
    };

    session.insert(db).await?;

    let refresh_claims = Claims::for_refresh(user_id.to_string(), jti, issued_at);
    Ok(refresh_claims)
}

fn issue_refresh_cookie(
    refresh_token: &str,
    cookie_secure: bool,
) -> Cookie<'static> {
    let mut cookie = Cookie::new("refresh_token", refresh_token.to_string());
    cookie.set_http_only(true);
    cookie.set_secure(cookie_secure);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");
    cookie.set_max_age(time::Duration::days(30));
    cookie
}

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
    jar: PrivateCookieJar,
    Json(req): Json<RegisterRequest>,
) -> Result<(PrivateCookieJar, Json<AuthResponse>), AppError> {
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

    let now = Utc::now().fixed_offset();
    let refresh_claims = create_refresh_session(&state.db, created.id, now).await?;

    let refresh_token = crate::jwt::encode(
        &refresh_claims,
        &state.config.jwt_secret,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create refresh token: {}", e)))?;

    let access_token = issue_access_token(&created.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create JWT: {}", e)))?;

    let refresh_cookie = issue_refresh_cookie(&refresh_token, state.config.cookie_secure);
    let jar = jar.add(refresh_cookie);

    Ok((jar, Json(AuthResponse {
        token: access_token,
        user: UserResponse::from(created),
    })))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<(PrivateCookieJar, Json<AuthResponse>), AppError> {
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

    let now = Utc::now().fixed_offset();
    let refresh_claims = create_refresh_session(&state.db, user.id, now).await?;

    let refresh_token = crate::jwt::encode(
        &refresh_claims,
        &state.config.jwt_secret,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create refresh token: {}", e)))?;

    let access_token = issue_access_token(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create JWT: {}", e)))?;

    let refresh_cookie = issue_refresh_cookie(&refresh_token, state.config.cookie_secure);
    let jar = jar.add(refresh_cookie);

    Ok((jar, Json(AuthResponse {
        token: access_token,
        user: UserResponse::from(user),
    })))
}

pub async fn logout() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "message": "Logged out successfully" }))
}
