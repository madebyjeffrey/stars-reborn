// Library module for integration tests
pub mod config;
pub mod db;
pub mod error;
pub mod features;
pub mod jwt;
pub mod middleware;

pub use config::Config;
pub use jwt::{Claims, TokenType};

// Re-export AppState type so it's available through the lib
// When compiled as a library, use this version; when as binary, main.rs version is used
use cookie::Key;
use axum::extract::FromRef;
use sea_orm::DatabaseConnection;

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




