# Backend DB Integration Tests

These tests are opt-in and run only when `RUN_DB_TESTS=1` is set.

Environment variables:

- `TEST_DATABASE_URL` (preferred for tests)
- `DATABASE_URL` (fallback if `TEST_DATABASE_URL` is unset or blank)

Example:

```zsh
export TEST_DATABASE_URL='postgres://postgres:postgres@localhost:5432/stars_reborn_test'
export RUN_DB_TESTS=1
cargo test --test db_smoke
```

The helper in `tests/common/mod.rs` resets the `public` schema and re-runs migrations before each test run.

