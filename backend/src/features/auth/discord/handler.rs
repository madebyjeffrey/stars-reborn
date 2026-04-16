use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar, SameSite};
use chrono::Utc;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::AppError,
    features::users::model::{self, Entity as UserEntity},
    jwt::issue_access_token,
    AppState,
};


pub async fn discord_login(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> (PrivateCookieJar, Redirect) {
    let client = BasicClient::new(ClientId::new(state.config.discord_client_id.clone()))
        .set_client_secret(ClientSecret::new(state.config.discord_client_secret.clone()))
        .set_auth_uri(AuthUrl::new("https://discord.com/api/oauth2/authorize".to_string()).unwrap())
        .set_token_uri(TokenUrl::new("https://discord.com/api/oauth2/token".to_string()).unwrap())
        .set_redirect_uri(RedirectUrl::new(state.config.discord_redirect_url.clone()).unwrap());
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

fn validate_csrf_state(
    jar: PrivateCookieJar,
    provided_state: Option<&str>,
) -> Result<PrivateCookieJar, AppError> {
    let csrf_cookie = jar
        .get("oauth_csrf")
        .ok_or_else(|| AppError::Auth("Missing CSRF cookie".to_string()))?;

    let expected = csrf_cookie.value();
    let provided = provided_state
        .ok_or_else(|| AppError::Auth("Missing state parameter".to_string()))?;

    if expected != provided {
        return Err(AppError::Auth("CSRF token mismatch".to_string()));
    }

    // Consume the CSRF cookie so it cannot be replayed.
    Ok(jar.remove(Cookie::from("oauth_csrf")))
}

pub async fn discord_callback(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Query(params): Query<CallbackQuery>,
) -> Result<(PrivateCookieJar, Redirect), AppError> {
    let jar = validate_csrf_state(jar, params.state.as_deref())?;

    let client = BasicClient::new(ClientId::new(state.config.discord_client_id.clone()))
        .set_client_secret(ClientSecret::new(state.config.discord_client_secret.clone()))
        .set_auth_uri(AuthUrl::new("https://discord.com/api/oauth2/authorize".to_string()).unwrap())
        .set_token_uri(TokenUrl::new("https://discord.com/api/oauth2/token".to_string()).unwrap())
        .set_redirect_uri(RedirectUrl::new(state.config.discord_redirect_url.clone()).unwrap());
    let http_client = oauth2::reqwest::ClientBuilder::new()
        .redirect(oauth2::reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to build OAuth HTTP client: {}", e)))?;

    let token_result = client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(&http_client)
        .await
        .map_err(|e| AppError::Auth(format!("OAuth token exchange failed: {}", e)))?;

    let access_token = token_result.access_token().secret().clone();

    let discord_user: DiscordUser = ::reqwest::Client::new()
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
        // Use Discord ID as username to guarantee uniqueness
        let username = format!("discord_{}", &discord_user.id);
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
        new_user.insert(&state.db).await.map_err(|e| {
            // Handle unique constraint violations gracefully
            if let sea_orm::DbErr::Custom(msg) = &e {
                if msg.contains("unique constraint") || msg.contains("UNIQUE constraint failed") {
                    return AppError::Conflict("Unable to create Discord user: username already exists".to_string());
                }
            }
            AppError::Database(e)
        })?
    };

    let token = issue_access_token(&user.id.to_string(), &state.config.jwt_secret)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create JWT: {}", e)))?;

    // Set JWT as HTTP-only cookie instead of query parameter to prevent token leakage
    let mut auth_cookie = Cookie::new("auth_token", token);
    auth_cookie.set_http_only(true);
    auth_cookie.set_secure(true);
    auth_cookie.set_same_site(SameSite::Lax);
    auth_cookie.set_path("/");
    auth_cookie.set_max_age(time::Duration::days(7));

    let jar = jar.add(auth_cookie);

    // Redirect to frontend callback page with just a success indicator
    // The JWT is transmitted securely via HTTP-only cookie, not in URL
    let redirect_url = format!("{}/auth/discord/callback", state.config.frontend_url);
    Ok((jar, Redirect::temporary(&redirect_url)))
}

#[cfg(test)]
mod tests {
    use super::validate_csrf_state;
    use crate::error::AppError;
    use axum_extra::extract::cookie::{Cookie, Key, PrivateCookieJar};

    fn new_private_jar() -> PrivateCookieJar {
        PrivateCookieJar::new(Key::generate())
    }

    fn jar_with_csrf(value: &str) -> PrivateCookieJar {
        new_private_jar().add(Cookie::new("oauth_csrf", value.to_string()))
    }

    #[test]
    fn validate_csrf_state_rejects_missing_csrf_cookie() {
        let err = validate_csrf_state(new_private_jar(), Some("state-value"))
            .expect_err("missing CSRF cookie should fail");

        match err {
            AppError::Auth(msg) => assert_eq!(msg, "Missing CSRF cookie"),
            other => panic!("expected auth error, got {other:?}"),
        }
    }

    #[test]
    fn validate_csrf_state_rejects_missing_state_parameter() {
        let err = validate_csrf_state(jar_with_csrf("expected-state"), None)
            .expect_err("missing state should fail");

        match err {
            AppError::Auth(msg) => assert_eq!(msg, "Missing state parameter"),
            other => panic!("expected auth error, got {other:?}"),
        }
    }

    #[test]
    fn validate_csrf_state_rejects_state_mismatch() {
        let err = validate_csrf_state(jar_with_csrf("expected-state"), Some("different-state"))
            .expect_err("mismatched state should fail");

        match err {
            AppError::Auth(msg) => assert_eq!(msg, "CSRF token mismatch"),
            other => panic!("expected auth error, got {other:?}"),
        }
    }

    #[test]
    fn validate_csrf_state_accepts_match_and_consumes_cookie() {
        let jar = validate_csrf_state(jar_with_csrf("expected-state"), Some("expected-state"))
            .expect("matching state should pass");

        assert!(jar.get("oauth_csrf").is_none());
    }
}
