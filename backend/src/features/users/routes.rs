use axum::{routing::get, Router};

use crate::AppState;
use super::handler;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(handler::get_me).put(handler::update_me))
}
