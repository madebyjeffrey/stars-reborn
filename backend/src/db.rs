use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};

pub async fn connect(database_url: &str) -> Result<DatabaseConnection, sea_orm::DbErr> {
    let db = Database::connect(database_url).await?;
    Ok(db)
}

pub async fn run_migrations(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    Migrator::up(db, None).await
}
