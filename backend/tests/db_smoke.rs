mod common;

use sea_orm::{ConnectionTrait, DbBackend, Statement};

#[tokio::test]
async fn test_db_setup_runs_migrations() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping DB integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let db = common::setup_test_db().await?;

    // Queries succeed if migrations created the tables.
    db.query_all(Statement::from_string(
        DbBackend::Postgres,
        "SELECT 1 FROM users LIMIT 1".to_string(),
    ))
    .await?;

    db.query_all(Statement::from_string(
        DbBackend::Postgres,
        "SELECT 1 FROM api_tokens LIMIT 1".to_string(),
    ))
    .await?;

    Ok(())
}

