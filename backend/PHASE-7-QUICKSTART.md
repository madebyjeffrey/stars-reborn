# Phase 7 Integration Tests - Quick Start Guide

## Setup

### 1. Create Test Database

```bash
# Connect to your PostgreSQL instance
psql -U postgres

# Create test database
CREATE DATABASE stars_reborn_test;
CREATE USER stars_test WITH PASSWORD 'stars_test';
ALTER ROLE stars_test WITH LOGIN;
GRANT ALL PRIVILEGES ON DATABASE stars_reborn_test TO stars_test;

# Or use docker
docker run -d \
  --name postgres-test \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=stars_reborn_test \
  -p 5432:5432 \
  postgres:15
```

### 2. Set Environment Variables

```bash
export TEST_DATABASE_URL='postgres://postgres:postgres@localhost:5432/stars_reborn_test'
export RUN_DB_TESTS=1
export JWT_SECRET='0123456789abcdef0123456789abcdef'
export API_TOKEN_PEPPER='test-pepper-value-1234567890'
```

## Running Tests

### Run All Auth Integration Tests

```bash
cargo test --test auth_integration
```

### Run Specific Test

```bash
# Local login token generation
cargo test --test auth_integration test_local_login_returns_access_token_in_response

# Refresh token rotation
cargo test --test auth_integration test_refresh_token_rotation_creates_new_session

# Conflict mapping
cargo test --test auth_integration test_conflict_mapping_username_duplicate

# All rotation/replay tests
cargo test --test auth_integration test_refresh_token
```

### Run Tests with Output

```bash
# Show stdout/stderr
cargo test --test auth_integration -- --nocapture

# Show debug info
RUST_LOG=debug cargo test --test auth_integration -- --nocapture
```

### Run in Watch Mode (with cargo-watch)

```bash
# Install if needed
cargo install cargo-watch

# Watch and re-run on changes
cargo watch -x "test --test auth_integration"
```

## Test List

| Test | Purpose |
|------|---------|
| `test_local_login_returns_access_token_in_response` | Verify access token generation for login |
| `test_local_register_creates_user_and_issues_tokens` | Verify user registration |
| `test_refresh_token_rotation_creates_new_session` | Verify refresh session persistence |
| `test_refresh_token_replay_detection_prevents_reuse` | Verify rotation prevents replay attacks |
| `test_logout_revocation_prevents_token_reuse` | Verify revocation blocks token reuse |
| `test_bearer_token_validation_accepts_access_tokens` | Verify bearer token structure |
| `test_refresh_token_has_correct_ttl` | Verify 30-day refresh token TTL |
| `test_access_token_has_correct_ttl` | Verify 7-day access token TTL |
| `test_cookie_secure_flag_configuration` | Verify cookie security config |
| `test_conflict_mapping_username_duplicate` | Verify duplicate username returns 409 |
| `test_conflict_mapping_email_duplicate` | Verify duplicate email returns 409 |
| `test_multiple_refresh_sessions_per_user` | Verify multiple sessions per user |
| `test_refresh_session_last_used_at_tracking` | Verify session usage tracking |

## Troubleshooting

### Tests Skipped

If you see:
```
Skipping auth integration test. Set RUN_DB_TESTS=1 to enable.
```

Fix:
```bash
export RUN_DB_TESTS=1
```

### Database Connection Error

If you see connection errors:

1. Verify database is running:
   ```bash
   psql -h localhost -U postgres -d stars_reborn_test -c "SELECT 1"
   ```

2. Check connection string:
   ```bash
   export TEST_DATABASE_URL='postgres://postgres:postgres@localhost:5432/stars_reborn_test'
   ```

3. Verify migrations run:
   ```bash
   psql -h localhost -U postgres -d stars_reborn_test -c "SELECT * FROM users LIMIT 1"
   ```

### Type Errors During Compilation

If compilation fails, ensure you're using the latest code:
```bash
cargo clean
cargo build
```

## CI/CD Integration

For GitHub Actions, use:

```yaml
- name: Run Integration Tests
  env:
    TEST_DATABASE_URL: postgres://postgres:postgres@localhost/stars_reborn_test
    RUN_DB_TESTS: 1
    JWT_SECRET: 0123456789abcdef0123456789abcdef
    API_TOKEN_PEPPER: test-pepper-value-1234567890
  run: cargo test --test auth_integration
  
  services:
    postgres:
      image: postgres:15
      env:
        POSTGRES_PASSWORD: postgres
        POSTGRES_DB: stars_reborn_test
      options: >-
        --health-cmd pg_isready
        --health-interval 10s
        --health-timeout 5s
        --health-retries 5
```

## Coverage

The auth integration tests provide coverage for:

- ✅ Local login flow (token generation)
- ✅ User registration (conflict detection)
- ✅ Refresh token rotation (replay prevention)
- ✅ Logout revocation
- ✅ Bearer token validation
- ✅ Token TTL enforcement
- ✅ Cookie security configuration
- ✅ Conflict mapping to HTTP 409
- ✅ Multi-session support per user
- ✅ Session usage tracking

## Next Steps

After verifying all tests pass:

1. **Phase 8**: Add native client readiness tests
2. **BFF Integration**: Add backend-for-frontend flow tests
3. **Performance**: Add benchmark tests for token operations
4. **Security**: Add penetration test scenarios

