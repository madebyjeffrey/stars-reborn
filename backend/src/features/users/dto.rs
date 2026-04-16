use serde::Serialize;

use super::model;

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub discord_id: Option<String>,
    pub discord_username: Option<String>,
    pub discord_avatar: Option<String>,
}

impl From<model::Model> for UserResponse {
    fn from(u: model::Model) -> Self {
        Self {
            id: u.id.to_string(),
            username: u.username,
            email: u.email,
            discord_id: u.discord_id,
            discord_username: u.discord_username,
            discord_avatar: u.discord_avatar,
        }
    }
}

