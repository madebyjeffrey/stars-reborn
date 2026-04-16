use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError,
    features::api_tokens::model::{self, Entity as TokenEntity},
    middleware::auth::AuthUser,
    AppState,
};

#[derive(Serialize)]
pub struct TokenResponse {
    pub id: String,
    pub name: String,
    pub token: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
}

impl TokenResponse {
    pub fn from_model(m: model::Model, show_token: bool) -> Self {
        Self {
            id: m.id.to_string(),
            name: m.name,
            token: if show_token { Some(m.token) } else { None },
            created_at: m.created_at.to_rfc3339(),
            expires_at: m.expires_at.map(|t| t.to_rfc3339()),
            last_used_at: m.last_used_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
}

pub async fn list_tokens(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<TokenResponse>>, AppError> {
    let tokens = TokenEntity::find()
        .filter(model::Column::UserId.eq(auth_user.user_id))
        .all(&state.db)
        .await?;

    Ok(Json(
        tokens
            .into_iter()
            .map(|t| TokenResponse::from_model(t, false))
            .collect(),
    ))
}

pub async fn create_token(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<CreateTokenRequest>,
) -> Result<(StatusCode, Json<TokenResponse>), AppError> {
    let token_str = hex::encode(rand::random::<[u8; 32]>());

    let now = Utc::now().fixed_offset();
    let token = model::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(auth_user.user_id),
        name: Set(req.name),
        token: Set(token_str),
        created_at: Set(now),
        expires_at: Set(None),
        last_used_at: Set(None),
    };

    let created = token.insert(&state.db).await?;

    Ok((
        StatusCode::CREATED,
        Json(TokenResponse::from_model(created, true)),
    ))
}

pub async fn delete_token(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let token = TokenEntity::find_by_id(id)
        .filter(model::Column::UserId.eq(auth_user.user_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Token not found".to_string()))?;

    let active: model::ActiveModel = token.into();
    active.delete(&state.db).await?;

    Ok(StatusCode::NO_CONTENT)
}
