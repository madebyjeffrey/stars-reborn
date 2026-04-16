use axum::{extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chrono::Utc;
use jsonwebtoken::{decode, DecodingKey, Validation};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError,
    features::api_tokens::{hash_api_token, model as api_token_model},
    AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub claims: Claims,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::Unauthorized)?;

        let bearer_token = bearer.token();

        let token_data = decode::<Claims>(
            bearer_token,
            &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            &Validation::default(),
        );

        if let Ok(token_data) = token_data {
            let user_id =
                Uuid::parse_str(&token_data.claims.sub).map_err(|_| AppError::Unauthorized)?;
            return Ok(AuthUser {
                user_id,
                claims: token_data.claims,
            });
        }

        let token_hash = hash_api_token(bearer_token, &state.config.jwt_secret);
        let token = api_token_model::Entity::find()
            .filter(api_token_model::Column::Token.eq(token_hash))
            .one(&state.db)
            .await?
            .ok_or(AppError::Unauthorized)?;

        let now = Utc::now().fixed_offset();
        if token.expires_at.is_some_and(|expires_at| expires_at <= now) {
            return Err(AppError::Unauthorized);
        }

        let mut token_active: api_token_model::ActiveModel = token.clone().into();
        token_active.last_used_at = Set(Some(now));
        token_active.update(&state.db).await?;

        Ok(AuthUser {
            user_id: token.user_id,
            claims: Claims {
                sub: token.user_id.to_string(),
                exp: usize::MAX,
                iat: now.timestamp() as usize,
            },
        })
    }
}
