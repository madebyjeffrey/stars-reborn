use axum::{extract::State, Json};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Deserializer, Serialize};

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

#[derive(Debug, PartialEq, Default)]
pub enum NullableFieldUpdate<T> {
    #[default]
    Unchanged,
    Clear,
    Set(T),
}

impl<'de, T> Deserialize<'de> for NullableFieldUpdate<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Self::Set(value),
            None => Self::Clear,
        })
    }
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    #[serde(default)]
    pub email: NullableFieldUpdate<String>,
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

    match req.email {
        NullableFieldUpdate::Unchanged => {}
        NullableFieldUpdate::Clear => active.email = Set(None),
        NullableFieldUpdate::Set(email) => active.email = Set(Some(email)),
    }

    active.updated_at = Set(Utc::now().fixed_offset());

    let updated = active.update(&state.db).await?;

    Ok(Json(UserResponse::from(updated)))
}

#[cfg(test)]
mod tests {
    use super::{NullableFieldUpdate, UpdateUserRequest};

    #[test]
    fn update_user_request_treats_missing_email_as_no_change() {
        let req: UpdateUserRequest =
            serde_json::from_str(r#"{"username":"pilot"}"#).expect("request should deserialize");

        assert_eq!(req.username.as_deref(), Some("pilot"));
        assert_eq!(req.email, NullableFieldUpdate::Unchanged);
    }

    #[test]
    fn update_user_request_treats_null_email_as_clear_request() {
        let req: UpdateUserRequest =
            serde_json::from_str(r#"{"email":null}"#).expect("request should deserialize");

        assert_eq!(req.email, NullableFieldUpdate::Clear);
    }

    #[test]
    fn update_user_request_treats_string_email_as_new_value() {
        let req: UpdateUserRequest = serde_json::from_str(r#"{"email":"captain@starsreborn.dev"}"#)
            .expect("request should deserialize");

        assert_eq!(
            req.email,
            NullableFieldUpdate::Set("captain@starsreborn.dev".to_string())
        );
    }
}
