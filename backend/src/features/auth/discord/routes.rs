use axum::{routing::get, Router};

use crate::AppState;
use super::handler;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::discord_login))
        .route("/callback", get(handler::discord_callback))
}
