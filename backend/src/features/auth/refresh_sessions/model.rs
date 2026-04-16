use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use crate::features::users;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "refresh_sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub jti: String,
    pub user_id: Uuid,
    pub expires_at: DateTimeWithTimeZone,
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub replaced_by: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub last_used_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "users::model::Entity",
        from = "Column::UserId",
        to = "users::model::Column::Id"
    )]
    User,
}

impl Related<users::model::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}


