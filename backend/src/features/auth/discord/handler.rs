use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar, SameSite};
use chrono::Utc;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::AppError,
    features::users::model::{self, Entity as UserEntity},
    AppState,
};

fn build_oauth_client(state: &AppState) -> BasicClient {
    BasicClient::new(
        ClientId::new(state.config.discord_client_id.clone()),
        Some(ClientSecret::new(
            state.config.discord_client_secret.clone(),
        )),
        AuthUrl::new("https://discord.com/api/oauth2/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://discord.com/api/oauth2/token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(state.config.discord_redirect_url.clone()).unwrap())
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
        exp: now + 86400 * 7,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub async fn discord_login(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> (PrivateCookieJar, Redirect) {
    let client = build_oauth_client(&state);
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .url();

    let mut csrf_cookie = Cookie::new("oauth_csrf", csrf_token.secret().clone());
    csrf_cookie.set_http_only(true);
    csrf_cookie.set_secure(true);
    csrf_cookie.set_same_site(SameSite::Lax);
    csrf_cookie.set_path("/");
    csrf_cookie.set_max_age(time::Duration::minutes(10));

    let jar = jar.add(csrf_cookie);
    (jar, Redirect::temporary(auth_url.as_str()))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    avatar: Option<String>,
    email: Option<String>,
}

pub async fn discord_callback(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Query(params): Query<CallbackQuery>,
) -> Result<(PrivateCookieJar, Redirect), AppError> {
    // Validate CSRF token before doing anything else
    let csrf_cookie = jar
        .get("oauth_csrf")
        .ok_or_else(|| AppError::Auth("Missing CSRF cookie".to_string()))?;

    let expected = csrf_cookie.value().to_string();
    let provided = params
        .state
        .as_deref()
        .ok_or_else(|| AppError::Auth("Missing state parameter".to_string()))?;

    if expected != provided {
        return Err(AppError::Auth("CSRF token mismatch".to_string()));
    }

    // Consume the CSRF cookie so it cannot be replayed
    let jar = jar.remove(Cookie::from("oauth_csrf"));

    let client = build_oauth_client(&state);

    let token_result = client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(async_http_client)
        .await
        .map_err(|e| AppError::Auth(format!("OAuth token exchange failed: {}", e)))?;

    let access_token = token_result.access_token().secret().clone();

    let discord_user: DiscordUser = reqwest::Client::new()
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to fetch Discord user: {}", e)))?
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse Discord user: {}", e)))?;

    let existing = UserEntity::find()
        .filter(model::Column::DiscordId.eq(&discord_user.id))
        .one(&state.db)
        .await?;

    let user = if let Some(user) = existing {
        user
    } else {
        let now = Utc::now().fixed_offset();
        let username = format!("discord_{}", &discord_user.username);
        let new_user = model::ActiveModel {
            id: Set(Uuid::new_v4()),
            username: Set(username),
            email: Set(discord_user.email),
            password_hash: Set(None),
            discord_id: Set(Some(discord_user.id)),
            discord_username: Set(Some(discord_user.username)),
            discord_avatar: Set(discord_user.avatar),
            created_at: Set(now),
            updated_at: Set(now),
        };
        new_user.insert(&state.db).await?
    };

    let token = create_jwt(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create JWT: {}", e)))?;

    let redirect_url = format!(
        "{}/auth/discord/callback?token={}",
        state.config.frontend_url, token
    );
    Ok((jar, Redirect::temporary(&redirect_url)))
}
