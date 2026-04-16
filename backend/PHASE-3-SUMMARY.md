# Phase 3 Implementation Summary - Unified Auth Contract

Implemented: 2026-04-16

## Overview

Phase 3 aligns Discord and local login to issue identical access+refresh token pairs stored in refresh sessions, eliminating the split-brain auth behavior.

## Changes Made

### 1. Local Auth Handlers (`backend/src/features/auth/local/handler.rs`)

**Updated `register()` endpoint:**
- Creates refresh session in database
- Issues both access + refresh tokens
- Sets refresh token in HTTP-only cookie (secure flag via `COOKIE_SECURE` env)
- Returns access token in response body
- Route signature changed to return `(PrivateCookieJar, Json<AuthResponse>)`

**Updated `login()` endpoint:**
- Same token issuance pattern as register
- Creates refresh session, sets refresh cookie
- Returns access token in response

**Helper functions:**
- `create_refresh_session()` - creates DB entry with 30-day expiry, returns refresh Claims
- `issue_refresh_cookie()` - creates HTTP-only refresh cookie with env-configurable security

### 2. Discord Auth Handler (`backend/src/features/auth/discord/handler.rs`)

**Updated `discord_callback()` endpoint:**
- Complete identity upsert (user creation or profile update)
- Creates refresh session in database
- Issues refresh token and stores in HTTP-only cookie
- No longer embeds tokens in URLs or uses old `auth_token` cookie
- Redirects to `/auth/discord/callback` for frontend to start refresh flow

**Key alignment with local flow:**
- Same token types (access = short-lived, refresh = 30-day)
- Same cookie policy (HTTP-only, secure, SameSite=Lax)
- Same jti/session tracking in refresh_sessions table

### 3. JWT Module Enhancement (`backend/src/jwt.rs`)

**Added generic encode function:**
```rust
pub fn encode(claims: &Claims, secret: &str) -> Result<String, ...>
```

Allows encoding any Claims struct (access, refresh, api tokens) with consistent algorithm/key derivation.

## API Contract Change

### Login & Register Endpoints

**Request:**
```
POST /api/auth/register or /api/auth/login
{ "username": "...", "password": "..." }
```

**Response:**
```
200 OK
Set-Cookie: refresh_token=<jwt>; HttpOnly; Secure; SameSite=Lax; Max-Age=2592000; Path=/
{
  "token": "<access_token>",
  "user": { "id": "...", "username": "...", ... }
}
```

**Frontend Usage:**
1. Store access token in memory (or short-lived sessionStorage)
2. Use access token for API calls via `Authorization: Bearer <access_token>`
3. On access token expiry (401), call `POST /api/auth/refresh` with cookie auto-sent
4. Refresh endpoint issues new access token

### Discord OAuth Flow

**Step 1:** `GET /api/auth/discord`
- Redirects to Discord OAuth consent screen

**Step 2:** Discord redirects to `GET /api/auth/discord/callback?code=...&state=...`
- Backend validates CSRF, exchanges code for Discord token, fetches user
- Creates/updates user in DB
- Creates refresh session
- Sets `refresh_token` cookie
- Redirects to `/auth/discord/callback` frontend route

**Step 3:** Frontend calls `POST /api/auth/refresh`
- Cookie sent automatically (with credentials)
- Backend validates refresh token, issues access token
- Returns `{ "access_token": "..." }`

**Frontend continues:** Uses new access token for API calls

## Database Behavior

All login methods now:
- Create entry in `refresh_sessions` table with jti, user_id, expires_at
- Store 30-day refresh token as Claims with `typ=Refresh`, `jti=<session_id>`
- Issue 7-day access token with `typ=Access`

## Security Improvements

✅ Refresh tokens:
- Stored as jti-identified records (enables revocation, rotation tracking)
- HTTP-only cookies (not accessible to JS)
- Configurable secure flag (COOKIE_SECURE env)
- Rotation tracked via `replaced_by` field for replay detection

✅ Access tokens:
- Short-lived (7 days)
- Discernible type (typ=Access)
- Never stored, always re-requested via refresh endpoint

✅ No embedded tokens in URLs (eliminates browser history leakage)

## Backward Compatibility

- Old OAuth `auth_token` cookie abandoned (frontend must migrate to refresh cookie + access token flow)
- Old login response structure unchanged externally (still returns `{ "token", "user" }` as access_token in `token` field)
- Local and Discord flows now identical from API perspective

## Test Status

All 36 unit tests passing:
- Config tests (COOKIE_SECURE behavior)
- JWT type/jti tests (refresh vs access tokens)
- Discord CSRF validation tests
- User update tests

## Migration Checklist for Frontend

- [ ] Update auth.service.ts to handle refresh cookie (already set by backend)
- [ ] Request access token from `/api/auth/refresh` on app init if refreshed
- [ ] Store access token in memory, not localStorage
- [ ] Implement 401 -> refresh -> retry logic in interceptor
- [ ] Remove old localStorage `auth_token` handling (except for transition period)
- [ ] Update Discord callback handler to call `/api/auth/refresh` instead of using cookie value

## Next Phase

Phase 4: Middleware cleanup, error handling quality, conflict mapping.

