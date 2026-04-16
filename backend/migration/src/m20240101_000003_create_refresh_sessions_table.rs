use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RefreshSessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RefreshSessions::Jti)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RefreshSessions::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(RefreshSessions::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RefreshSessions::RevokedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(RefreshSessions::ReplacedBy).string())
                    .col(
                        ColumnDef::new(RefreshSessions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RefreshSessions::LastUsedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_refresh_sessions_user_id")
                            .from(RefreshSessions::Table, RefreshSessions::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx_refresh_sessions_user_id")
                            .table(RefreshSessions::Table)
                            .col(RefreshSessions::UserId),
                    )
                    .index(
                        Index::create()
                            .name("idx_refresh_sessions_expires_at")
                            .table(RefreshSessions::Table)
                            .col(RefreshSessions::ExpiresAt),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RefreshSessions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum RefreshSessions {
    Table,
    Jti,
    UserId,
    ExpiresAt,
    RevokedAt,
    ReplacedBy,
    CreatedAt,
    LastUsedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

