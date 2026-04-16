use axum::{extract::State, Json};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Deserializer};

use crate::{
    error::AppError,
    features::users::{dto::UserResponse, model::{self, Entity as UserEntity}},
    middleware::auth::AuthUser,
    AppState,
};

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

    let updated = active
        .update(&state.db)
        .await
        .map_err(map_user_unique_constraint_error)?;

    Ok(Json(UserResponse::from(updated)))
}

#[cfg(test)]
mod tests {
    use super::{NullableFieldUpdate, UpdateUserRequest};
    use super::map_user_unique_constraint_error;
    use crate::error::AppError;

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

    #[test]
    fn map_user_unique_constraint_error_detects_username_conflict() {
        let db_err = sea_orm::DbErr::Custom(
            "duplicate key value violates unique constraint \"users_username_key\"".to_string(),
        );

        let err = map_user_unique_constraint_error(db_err);
        match err {
            AppError::Conflict(msg) => assert_eq!(msg, "Username already taken"),
            other => panic!("expected Conflict error, got {other:?}"),
        }
    }

    #[test]
    fn map_user_unique_constraint_error_detects_email_conflict() {
        let db_err = sea_orm::DbErr::Custom(
            "duplicate key value violates unique constraint \"users_email_key\"".to_string(),
        );

        let err = map_user_unique_constraint_error(db_err);
        match err {
            AppError::Conflict(msg) => assert_eq!(msg, "Email already in use"),
            other => panic!("expected Conflict error, got {other:?}"),
        }
    }

    #[test]
    fn map_user_unique_constraint_error_handles_generic_db_errors() {
        let db_err = sea_orm::DbErr::Custom("connection refused".to_string());

        let err = map_user_unique_constraint_error(db_err);
        match err {
            AppError::Database(_) => {}, // expected
            other => panic!("expected Database error, got {other:?}"),
        }
    }

    #[test]
    fn map_user_unique_constraint_error_detects_username_conflict_case_insensitive() {
        let db_err = sea_orm::DbErr::Custom("Duplicate key value violates UNIQUE constraint \"users_username_key\"".to_string());

        let err = map_user_unique_constraint_error(db_err);
        match err {
            AppError::Conflict(msg) => assert_eq!(msg, "Username already taken"),
            other => panic!("expected Conflict error, got {other:?}"),
        }
    }
}
