use std::env;
use axum::http::HeaderValue;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_redirect_url: String,
    pub frontend_url: String,
    pub frontend_origin: HeaderValue,
    pub server_host: String,
    pub server_port: u16,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let frontend_url = env::var("FRONTEND_URL")
            .unwrap_or_else(|_| "http://localhost:4200".to_string());

        let frontend_origin = frontend_url
            .parse::<HeaderValue>()
            .map_err(|err| anyhow::anyhow!("Invalid FRONTEND_URL for CORS allow_origin ('{frontend_url}'): {err}"))?;

        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|err| anyhow::anyhow!("SERVER_PORT must be a valid u16: {err}"))?;

        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?,
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            discord_client_id: env::var("DISCORD_CLIENT_ID").unwrap_or_default(),
            discord_client_secret: env::var("DISCORD_CLIENT_SECRET").unwrap_or_default(),
            discord_redirect_url: env::var("DISCORD_REDIRECT_URL").unwrap_or_else(|_| {
                "http://localhost:3000/api/auth/discord/callback".to_string()
            }),
            frontend_url,
            frontend_origin,
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port,
        })
    }
}
