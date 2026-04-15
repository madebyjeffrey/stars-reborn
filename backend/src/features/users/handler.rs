use axum::{extract::State, Json};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppError,
    features::users::model::{self, Entity as UserEntity},
    middleware::auth::AuthUser,
    AppState,
};

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub discord_id: Option<String>,
    pub discord_username: Option<String>,
    pub discord_avatar: Option<String>,
}

impl From<model::Model> for UserResponse {
    fn from(u: model::Model) -> Self {
        Self {
            id: u.id.to_string(),
            username: u.username,
            email: u.email,
            discord_id: u.discord_id,
            discord_username: u.discord_username,
            discord_avatar: u.discord_avatar,
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
}

pub async fn get_me(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<UserResponse>, AppError> {
    let user = UserEntity::find_by_id(auth_user.user_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(Json(UserResponse::from(user)))
}

pub async fn update_me(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let user = UserEntity::find_by_id(auth_user.user_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let mut active: model::ActiveModel = user.into();

    if let Some(username) = req.username {
        active.username = Set(username);
    }

    if let Some(email) = req.email {
        active.email = Set(Some(email));
    }

    let updated = active.update(&state.db).await?;

    Ok(Json(UserResponse::from(updated)))
}
