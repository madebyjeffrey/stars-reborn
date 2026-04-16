use axum::{routing::post, Router};

use crate::AppState;
use super::handler;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/refresh", post(handler::refresh))
        .route("/logout", post(handler::logout))
}

