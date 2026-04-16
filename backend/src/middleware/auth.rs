use axum::{extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::{
    error::AppError,
    features::api_tokens::{hash_api_token, model as api_token_model},
    jwt::{decode_access_token, Claims, TokenType},
    AppState,
};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    #[allow(dead_code)]
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

        let token_data = decode_access_token(bearer_token, &state.config.jwt_secret);

        if let Ok(token_data) = token_data {
            // Accept both access and refresh tokens from middleware
            if !matches!(
                token_data.claims.typ,
                Some(TokenType::Access) | Some(TokenType::Refresh)
            ) {
                return Err(AppError::Unauthorized);
            }
            let user_id =
                Uuid::parse_str(&token_data.claims.sub).map_err(|_| AppError::Unauthorized)?;
            return Ok(AuthUser {
                user_id,
                claims: token_data.claims,
            });
        }

        let token_hash = hash_api_token(bearer_token, &state.config.api_token_pepper);

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
            claims: Claims::for_api_token(token.user_id.to_string(), now.timestamp() as usize),
        })
    }
}
