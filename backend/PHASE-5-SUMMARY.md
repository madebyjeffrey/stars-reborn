# Phase 5 Summary - API Token Pepper Separation (No Legacy Path)

## What changed

- Added required `API_TOKEN_PEPPER` to backend config in `backend/src/config.rs`.
- Decoupled API token hashing from `JWT_SECRET` for both creation and verification.
- Removed legacy compatibility path because the application has not been deployed yet.
- Updated env template in `backend/.env.example` to require a dedicated API token pepper.

## Code updates

- `backend/src/config.rs`
  - `Config` now uses a single `api_token_pepper: String` field.
  - `API_TOKEN_PEPPER` is required and must be non-empty.
  - No fallback to `JWT_SECRET` and no legacy pepper handling.
- `backend/src/features/api_tokens/mod.rs`
  - Kept deterministic `hash_api_token(raw_token, secret)` hashing helper.
  - Removed legacy hash-candidate compatibility helper.
- `backend/src/features/api_tokens/handler.rs`
  - New token creation hashes with `config.api_token_pepper`.
- `backend/src/middleware/auth.rs`
  - API token verification uses only `API_TOKEN_PEPPER` hash.
  - Removed dual-hash verification and rehash-on-use logic.

## Operational note

Since no production tokens exist yet, this is the simplest and safest approach:

1. Set `API_TOKEN_PEPPER` once per environment.
2. Keep `JWT_SECRET` dedicated to JWT signing only.
3. Rotate the two secrets independently in future operations.

## Validation

- Backend test suite passes after changes:
  - `cargo test -p stars-reborn-backend -- --nocapture`
  - Result: all tests passing.


