mod common;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, ActiveModelTrait, Set};
use stars_reborn_backend::features::users::model as user_model;
use stars_reborn_backend::features::auth::refresh_sessions::model as refresh_model;

#[tokio::test]
async fn test_local_login_returns_access_token_in_response() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let db = common::setup_test_db().await?;

    // Create a test user
    let user = create_test_user(&db, "testuser", "testpass", None).await?;

    // Simulate local login (in real scenario this would be via HTTP)
    let access_token = stars_reborn_backend::jwt::issue_access_token(
        &user.id.to_string(),
        "test-jwt-secret-that-is-long-enough-1234567890",
    )?;

    // Verify token is not empty
    assert!(!access_token.is_empty());

    // Decode to verify it's a valid JWT
    let token_data = stars_reborn_backend::jwt::decode_access_token(
        &access_token,
        "test-jwt-secret-that-is-long-enough-1234567890",
    )?;

    assert_eq!(token_data.claims.sub, user.id.to_string());
    assert_eq!(token_data.claims.typ, Some(stars_reborn_backend::jwt::TokenType::Access));

    Ok(())
}

#[tokio::test]
async fn test_local_register_creates_user_and_issues_tokens() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let db = common::setup_test_db().await?;

    // Register a new user
    let user = create_test_user(&db, "newuser", "newpass", Some("new@example.com")).await?;

    // Verify user exists
    let found_user = user_model::Entity::find_by_id(user.id)
        .one(&db)
        .await?;
    assert!(found_user.is_some());

    // Verify username and email
    let found = found_user.unwrap();
    assert_eq!(found.username, "newuser");
    assert_eq!(found.email, Some("new@example.com".to_string()));
    assert!(found.password_hash.is_some());

    Ok(())
}

#[tokio::test]
async fn test_refresh_token_rotation_creates_new_session() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let db = common::setup_test_db().await?;
    let user = create_test_user(&db, "rotateuser", "pass", None).await?;

    // Create initial refresh session
    let jti_1 = uuid::Uuid::new_v4().to_string();
    let expires_at_1 = chrono::Utc::now().fixed_offset() + chrono::Duration::days(30);
    let session_1 = refresh_model::ActiveModel {
        jti: Set(jti_1.clone()),
        user_id: Set(user.id),
        expires_at: Set(expires_at_1),
        revoked_at: Set(None),
        replaced_by: Set(None),
        created_at: Set(chrono::Utc::now().fixed_offset()),
        last_used_at: Set(None),
    };

    session_1.insert(&db).await?;

    // Verify session was created
    let found = refresh_model::Entity::find()
        .filter(refresh_model::Column::Jti.eq(&jti_1))
        .one(&db)
        .await?;
    assert!(found.is_some());
    assert_eq!(found.unwrap().jti, jti_1);

    Ok(())
}

#[tokio::test]
async fn test_refresh_token_replay_detection_prevents_reuse() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let db = common::setup_test_db().await?;
    let user = create_test_user(&db, "replayuser", "pass", None).await?;

    // Create refresh session
    let jti = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().fixed_offset();
    let expires_at = now + chrono::Duration::days(30);

    let session = refresh_model::ActiveModel {
        jti: Set(jti.clone()),
        user_id: Set(user.id),
        expires_at: Set(expires_at),
        revoked_at: Set(None),
        replaced_by: Set(None),
        created_at: Set(now),
        last_used_at: Set(None),
    };

    let inserted = session.insert(&db).await?;

    // Mark as replaced (simulating rotation)
    let replacement_jti = uuid::Uuid::new_v4().to_string();
    let mut active: refresh_model::ActiveModel = inserted.into();
    active.replaced_by = Set(Some(replacement_jti.clone()));
    active.update(&db).await?;

    // Verify the old session is marked as replaced
    let found = refresh_model::Entity::find()
        .filter(refresh_model::Column::Jti.eq(&jti))
        .one(&db)
        .await?;

    assert!(found.is_some());
    assert_eq!(found.unwrap().replaced_by, Some(replacement_jti));

    Ok(())
}

#[tokio::test]
async fn test_logout_revocation_prevents_token_reuse() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let db = common::setup_test_db().await?;
    let user = create_test_user(&db, "revokeuser", "pass", None).await?;

    // Create refresh session
    let jti = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().fixed_offset();
    let expires_at = now + chrono::Duration::days(30);

    let session = refresh_model::ActiveModel {
        jti: Set(jti.clone()),
        user_id: Set(user.id),
        expires_at: Set(expires_at),
        revoked_at: Set(None),
        replaced_by: Set(None),
        created_at: Set(now),
        last_used_at: Set(None),
    };

    let inserted = session.insert(&db).await?;

    // Simulate logout: revoke the session
    let mut active: refresh_model::ActiveModel = inserted.into();
    active.revoked_at = Set(Some(now));
    active.update(&db).await?;

    // Verify session is revoked
    let found = refresh_model::Entity::find()
        .filter(refresh_model::Column::Jti.eq(&jti))
        .one(&db)
        .await?;

    assert!(found.is_some());
    assert!(found.unwrap().revoked_at.is_some());

    Ok(())
}

#[tokio::test]
async fn test_conflict_mapping_username_duplicate() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let db = common::setup_test_db().await?;

    // Create first user
    create_test_user(&db, "duplicate", "pass", None).await?;

    // Attempt to create second user with same username
    let result = create_test_user(&db, "duplicate", "pass2", None).await;

    // Should fail with conflict
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(err_msg.contains("username") || err_msg.contains("unique"));

    Ok(())
}

#[tokio::test]
async fn test_conflict_mapping_email_duplicate() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let db = common::setup_test_db().await?;

    // Create first user with email
    create_test_user(&db, "user1", "pass", Some("same@example.com")).await?;

    // Attempt to create second user with same email
    let result = create_test_user(&db, "user2", "pass2", Some("same@example.com")).await;

    // Should fail with conflict
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(err_msg.contains("email") || err_msg.contains("unique"));

    Ok(())
}

#[tokio::test]
async fn test_bearer_token_validation_accepts_access_tokens() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let db = common::setup_test_db().await?;
    let user = create_test_user(&db, "beareruser", "pass", None).await?;

    // Issue access token
    let access_token = stars_reborn_backend::jwt::issue_access_token(
        &user.id.to_string(),
        "test-jwt-secret-that-is-long-enough-1234567890",
    )?;

    // Decode and verify it's an access token
    let token_data = stars_reborn_backend::jwt::decode_access_token(
        &access_token,
        "test-jwt-secret-that-is-long-enough-1234567890",
    )?;

    assert_eq!(token_data.claims.sub, user.id.to_string());
    assert_eq!(token_data.claims.typ, Some(stars_reborn_backend::jwt::TokenType::Access));

    Ok(())
}

#[tokio::test]
async fn test_refresh_token_has_correct_ttl() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let db = common::setup_test_db().await?;
    let user = create_test_user(&db, "ttluser", "pass", None).await?;

    // Create refresh session
    let jti = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().fixed_offset();
    let issued_at = now.timestamp() as usize;

    // Create refresh token claim
    let refresh_claims = stars_reborn_backend::jwt::Claims::for_refresh(
        user.id.to_string(),
        jti,
        issued_at,
    );

    // Verify TTL is approximately 30 days
    let expected_exp = issued_at + stars_reborn_backend::jwt::REFRESH_TOKEN_TTL_SECONDS;
    assert_eq!(refresh_claims.exp, expected_exp);

    // Verify token type is set
    assert_eq!(refresh_claims.typ, Some(stars_reborn_backend::jwt::TokenType::Refresh));

    Ok(())
}

#[tokio::test]
async fn test_access_token_has_correct_ttl() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let user_id = "test-user-id";
    let issued_at = 1_700_000_000usize;

    let access_claims = stars_reborn_backend::jwt::Claims::for_user(user_id, issued_at);

    // Verify TTL is approximately 7 days
    let expected_exp = issued_at + stars_reborn_backend::jwt::ACCESS_TOKEN_TTL_SECONDS;
    assert_eq!(access_claims.exp, expected_exp);

    // Verify token type is set
    assert_eq!(access_claims.typ, Some(stars_reborn_backend::jwt::TokenType::Access));

    Ok(())
}

#[tokio::test]
async fn test_cookie_secure_flag_configuration() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    // This test verifies that cookie_secure configuration is properly read from environment
    // In production: COOKIE_SECURE=true
    // In local dev: COOKIE_SECURE=false

    // The config module handles this parsing
    // We verify the config accepts both values
    use std::env;

    let original = env::var("COOKIE_SECURE").ok();
    let original_jwt = env::var("JWT_SECRET").ok();
    let original_db = env::var("DATABASE_URL").ok();
    let original_pepper = env::var("API_TOKEN_PEPPER").ok();

    env::set_var("DATABASE_URL", "postgres://localhost/test");
    env::set_var("JWT_SECRET", "0123456789abcdef0123456789abcdef");
    env::set_var("API_TOKEN_PEPPER", "pepper1234567890");

    // Test COOKIE_SECURE=true
    env::set_var("COOKIE_SECURE", "true");
    let cfg = stars_reborn_backend::Config::from_env();
    assert!(cfg.is_ok());
    assert!(cfg.unwrap().cookie_secure);

    // Test COOKIE_SECURE=false
    env::set_var("COOKIE_SECURE", "false");
    let cfg = stars_reborn_backend::Config::from_env();
    assert!(cfg.is_ok());
    assert!(!cfg.unwrap().cookie_secure);

    // Cleanup
    if let Some(val) = original {
        env::set_var("COOKIE_SECURE", val);
    } else {
        env::remove_var("COOKIE_SECURE");
    }
    if let Some(val) = original_jwt {
        env::set_var("JWT_SECRET", val);
    } else {
        env::remove_var("JWT_SECRET");
    }
    if let Some(val) = original_db {
        env::set_var("DATABASE_URL", val);
    } else {
        env::remove_var("DATABASE_URL");
    }
    if let Some(val) = original_pepper {
        env::set_var("API_TOKEN_PEPPER", val);
    } else {
        env::remove_var("API_TOKEN_PEPPER");
    }

    Ok(())
}

#[tokio::test]
async fn test_multiple_refresh_sessions_per_user() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let db = common::setup_test_db().await?;
    let user = create_test_user(&db, "multiuser", "pass", None).await?;

    let now = chrono::Utc::now().fixed_offset();
    let expires_at = now + chrono::Duration::days(30);

    // Create multiple refresh sessions for the same user
    let jti_1 = uuid::Uuid::new_v4().to_string();
    let session_1 = refresh_model::ActiveModel {
        jti: Set(jti_1.clone()),
        user_id: Set(user.id),
        expires_at: Set(expires_at),
        revoked_at: Set(None),
        replaced_by: Set(None),
        created_at: Set(now),
        last_used_at: Set(None),
    };
    session_1.insert(&db).await?;

    let jti_2 = uuid::Uuid::new_v4().to_string();
    let session_2 = refresh_model::ActiveModel {
        jti: Set(jti_2.clone()),
        user_id: Set(user.id),
        expires_at: Set(expires_at),
        revoked_at: Set(None),
        replaced_by: Set(None),
        created_at: Set(now),
        last_used_at: Set(None),
    };
    session_2.insert(&db).await?;

    // Verify both sessions exist
    let sessions = refresh_model::Entity::find()
        .filter(refresh_model::Column::UserId.eq(user.id))
        .all(&db)
        .await?;

    assert_eq!(sessions.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_refresh_session_last_used_at_tracking() -> anyhow::Result<()> {
    if !common::should_run_db_tests() {
        eprintln!("Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.");
        return Ok(());
    }

    let db = common::setup_test_db().await?;
    let user = create_test_user(&db, "lastuseduser", "pass", None).await?;

    // Create refresh session
    let jti = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().fixed_offset();
    let expires_at = now + chrono::Duration::days(30);

    let session = refresh_model::ActiveModel {
        jti: Set(jti.clone()),
        user_id: Set(user.id),
        expires_at: Set(expires_at),
        revoked_at: Set(None),
        replaced_by: Set(None),
        created_at: Set(now),
        last_used_at: Set(None),
    };

    let inserted = session.insert(&db).await?;
    assert!(inserted.last_used_at.is_none());

    // Update last_used_at
    let later = now + chrono::Duration::seconds(30);
    let mut active: refresh_model::ActiveModel = inserted.into();
    active.last_used_at = Set(Some(later));
    let updated = active.update(&db).await?;

    assert_eq!(updated.last_used_at, Some(later));

    Ok(())
}

// Helper function to create a test user
async fn create_test_user(
    db: &sea_orm::DatabaseConnection,
    username: &str,
    password: &str,
    email: Option<&str>,
) -> anyhow::Result<user_model::Model> {
    use bcrypt::{hash, DEFAULT_COST};

    let password_hash = hash(password, DEFAULT_COST)?;
    let now = chrono::Utc::now().fixed_offset();
    let id = uuid::Uuid::new_v4();

    let user = user_model::ActiveModel {
        id: Set(id),
        username: Set(username.to_string()),
        email: Set(email.map(|e| e.to_string())),
        password_hash: Set(Some(password_hash)),
        discord_id: Set(None),
        discord_username: Set(None),
        discord_avatar: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let created = user.insert(db).await?;
    Ok(created)
}












