# Phase 2 Implementation Summary - Refresh Token Foundation

Implemented: 2026-04-16

## Overview

Phase 2 introduces first-party token lifecycle primitives (access + refresh tokens) with rotation and revocation support. Both Discord and local auth will use these in Phase 3.

## Changes Made

### 1. JWT Claims Extension (`backend/src/jwt.rs`)

Added typed token support:
- New `TokenType` enum: `Access`, `Refresh`
- Extended `Claims` struct with:
  - `typ: Option<TokenType>` - token type discrimination
  - `jti: Option<String>` - unique token ID for session tracking
- New claim constructors:
  - `Claims::for_refresh(user_id, jti, issued_at)` - 30-day refresh tokens
  - `Claims::for_access_with_jti(user_id, jti, issued_at)` - access tokens with session ID
- Existing JWT functions remain backward-compatible

Token TTL constants:
- Access: 7 days (existing)
- Refresh: 30 days (new)

### 2. Refresh Sessions Storage (`backend/migration/src/m20240101_000003_create_refresh_sessions_table.rs`)

Created `refresh_sessions` table:
- `jti` (string, PK) - unique session identifier
- `user_id` (UUID, FK) - references users
- `expires_at` (timestamp) - session expiration
- `revoked_at` (timestamp, nullable) - when revoked (logout)
- `replaced_by` (string, nullable) - points to new jti on rotation (replay detection)
- `created_at` (timestamp) - session creation
- `last_used_at` (timestamp, nullable) - track usage for analysis

Indexes on `user_id` and `expires_at` for queries.

### 3. Refresh Sessions Feature (`backend/src/features/auth/refresh_sessions/*`)

New feature slice with three modules:

**model.rs:**
- `RefreshSessions::Model` - SeaORM entity for refresh_sessions table
- Relationships to Users

**handler.rs:**
- `POST /api/auth/refresh` endpoint
  - Validates incoming token is a refresh token (typ == Refresh)
  - Checks session exists and is not revoked
  - Detects replay attacks (if `replaced_by` is set)
  - Issues new access token
  - Updates `last_used_at` for analytics
- `POST /api/auth/logout` endpoint
  - Revokes current refresh session by setting `revoked_at`
  - Works with any authenticated token (has jti)

**routes.rs:**
- Exposes refresh and logout routes under `/api/auth` path

### 4. Middleware Updates (`backend/src/middleware/auth.rs`)

Updated auth extractor to accept both access and refresh tokens:
- Validates token type is one of `Access` or `Refresh`
- Maintains backward compatibility with API tokens (typ = None)
- Preserves claims.jti and claims.typ for handler inspection

### 5. Integration (`backend/src/main.rs`, `backend/src/features/auth/mod.rs`)

- Registered refresh_sessions routes under `/api/auth` alongside local/discord routes
- Exposed refresh_sessions module publicly

## Test Coverage

Unit tests verify:
- JWT claims with typ and jti serialize/deserialize correctly
- Token type discrimination (access vs refresh)
- Correct TTL values for each token type
- Access and refresh tokens have different expiration windows
- Refresh token session structure validates jti and typ

All 36 tests passing (33 pre-Phase-2 + 3 new token type tests).

## Database Migration

New migration registers automatically in migrator. Run via:
```zsh
cargo test  # auto-migrates on test DB
# or in prod: cargo run  # applies migrations on startup
```

## Backward Compatibility

- Existing `Claims::for_user()` and `Claims::for_api_token()` work unchanged
- Access tokens default to `typ=Access` for future discernibility
- API tokens retain `typ=None` (phase 5 to fix)
- Old JWT decode still works (typ and jti are optional)

## Next: Phase 3

Local login/register and Discord callback will use new endpoint contract:
- Issue both access token and refresh token on login
- Store session in refresh_sessions table
- Set refresh token in HTTP-only cookie (for web client)
- Return access token for API use

## API Endpoints (Phase 2)

Ready for use but not yet connected to login flows:

- `POST /api/auth/refresh` - requires bearer refresh token
  - Response: `{ "access_token": "..." }`
- `POST /api/auth/logout` - requires any authenticated bearer token
  - Response: `{ "message": "Logged out successfully" }`

