mod config;
mod db;
mod error;
mod features;
mod middleware;

use axum::{routing::get, Json, Router};
use sea_orm::DatabaseConnection;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cookie::Key;
use axum::extract::FromRef;
use config::Config;
use features::{
    api_tokens::routes as token_routes,
    auth::{discord::routes as discord_routes, local::routes as local_routes},
    users::routes as user_routes,
};

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Config,
    pub cookie_key: Key,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "Stars Reborn" }))
}

#[tokio::main]
async fn main() {
    let purge_plaintext_api_tokens = std::env::args().any(|arg| arg == "--purge-plaintext-api-tokens");

    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "stars_reborn_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();

    let db = db::connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    db::run_migrations(&db)
        .await
        .expect("Failed to run database migrations");

    if purge_plaintext_api_tokens {
        let removed = features::api_tokens::purge_non_hashed_tokens(&db)
            .await
            .expect("Failed to purge non-hashed API tokens");
        tracing::info!(
            "Purged {} non-hashed API token(s). Exiting as requested.",
            removed
        );
        return;
    }

    features::api_tokens::assert_all_tokens_hashed(&db)
        .await
        .expect("Found non-hashed API tokens in database");

    let state = AppState {
        db,
        cookie_key: Key::derive_from(config.jwt_secret.as_bytes()),
        config: config.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(
            config
                .frontend_url
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_credentials(true);

    let app = Router::new()
        .route("/api/health", get(health_check))
        .nest("/api/auth", local_routes::routes())
        .nest("/api/auth/discord", discord_routes::routes())
        .nest("/api/users", user_routes::routes())
        .nest("/api/tokens", token_routes::routes())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", config.server_host, config.server_port);
    tracing::info!("Starting Stars Reborn backend on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}
