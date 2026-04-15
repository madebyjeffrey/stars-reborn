use axum::{routing::post, Router};

use crate::AppState;
use super::handler;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(handler::register))
        .route("/login", post(handler::login))
        .route("/logout", post(handler::logout))
}
