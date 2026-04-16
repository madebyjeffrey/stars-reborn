# Backend DB Integration Tests

These tests are opt-in and run only when `RUN_DB_TESTS=1` is set.

Environment variables:

- `TEST_DATABASE_URL` (preferred for tests)
- `DATABASE_URL` (fallback if `TEST_DATABASE_URL` is unset or blank)

## Available Test Suites

### db_smoke - Basic database and migration verification

Verifies that migrations run successfully and create required tables.

```zsh
export TEST_DATABASE_URL='postgres://postgres:postgres@localhost:5432/stars_reborn_test'
export RUN_DB_TESTS=1
cargo test --test db_smoke
```

### auth_integration - Authentication flow integration tests (Phase 7)

Comprehensive integration tests for:
- Local login token generation
- User registration with conflict detection
- Refresh token rotation and replay prevention
- Logout revocation behavior
- HTTP-only cookie security configuration
- Multiple refresh sessions per user
- Session usage tracking

Tests include scenarios for:
- Bearer token validation (access tokens)
- Refresh token TTL enforcement
- Access token TTL enforcement
- Conflict mapping (`409` status for duplicate username/email)
- Environment-based cookie security flags

```zsh
export TEST_DATABASE_URL='postgres://postgres:postgres@localhost:5432/stars_reborn_test'
export RUN_DB_TESTS=1
cargo test --test auth_integration
```

## Test Infrastructure

The helper in `tests/common/mod.rs` provides:
- Database connection setup with URL resolution
- Schema reset (drops and recreates the `public` schema)
- Full migration execution before each test

All tests are async and support proper test isolation via database cleanup.

## Running All Integration Tests

```zsh
export TEST_DATABASE_URL='postgres://postgres:postgres@localhost:5432/stars_reborn_test'
export RUN_DB_TESTS=1
cargo test --test '*'
```

