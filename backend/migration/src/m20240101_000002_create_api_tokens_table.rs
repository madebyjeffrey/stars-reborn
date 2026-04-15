use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiTokens::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ApiTokens::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(ApiTokens::UserId).uuid().not_null())
                    .col(ColumnDef::new(ApiTokens::Name).string().not_null())
                    .col(
                        ColumnDef::new(ApiTokens::Token)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(ApiTokens::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ApiTokens::ExpiresAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(ApiTokens::LastUsedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_api_tokens_user_id")
                            .from(ApiTokens::Table, ApiTokens::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiTokens::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ApiTokens {
    Table,
    Id,
    UserId,
    Name,
    Token,
    CreatedAt,
    ExpiresAt,
    LastUsedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
