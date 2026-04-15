use axum::{routing::get, Router};

use crate::AppState;
use super::handler;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::list_tokens).post(handler::create_token))
        .route("/:id", axum::routing::delete(handler::delete_token))
}
