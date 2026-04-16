use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

pub fn should_run_db_tests() -> bool {
    std::env::var("RUN_DB_TESTS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub async fn setup_test_db() -> anyhow::Result<DatabaseConnection> {
    let database_url = resolve_test_database_url_from_env()?;
    let db = Database::connect(&database_url).await?;

    reset_schema(&db).await?;
    Migrator::up(&db, None).await?;

    Ok(db)
}

fn resolve_test_database_url_from_env() -> anyhow::Result<String> {
    if let Some(url) = std::env::var("TEST_DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(url);
    }

    std::env::var("DATABASE_URL").map_err(|_| {
        anyhow::anyhow!(
            "DATABASE_URL must be set (or provide TEST_DATABASE_URL for integration tests)"
        )
    })
}

async fn reset_schema(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    db.execute(Statement::from_string(
        DbBackend::Postgres,
        "DROP SCHEMA IF EXISTS public CASCADE".to_string(),
    ))
    .await?;

    db.execute(Statement::from_string(
        DbBackend::Postgres,
        "CREATE SCHEMA public".to_string(),
    ))
    .await?;

    Ok(())
}


