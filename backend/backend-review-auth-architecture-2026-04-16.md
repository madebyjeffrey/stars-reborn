# Backend Review and Auth Architecture Guidance (2026-04-16)

## Scope

This report captures issues found in the backend review, recommended mitigations, and auth model guidance aligned with:

- Pragmatic vertical slices
- Discord as the primary user type
- Future support for web and native clients
- Optional future BFF (Backend-for-Frontend)

## Findings and Ideal Mitigations

### 1) OAuth cookie auth is not accepted by API auth middleware (Critical)

- **Where**
  - Cookie issued in `backend/src/features/auth/discord/handler.rs` (JWT cookie `auth_token`)
  - API auth extractor reads only `Authorization: Bearer` in `backend/src/middleware/auth.rs`
- **Impact**
  - Discord login can appear successful, but subsequent API calls fail unless frontend manually sends a bearer token.
  - Browser cookie flow and API auth contract are inconsistent.
- **Ideal mitigation**
  - Pick one canonical API auth contract and enforce it consistently.
  - Recommended: API accepts bearer access tokens only; if cookie is used in browser, add a dedicated path that converts session/cookie state into API token usage (BFF or explicit token refresh endpoint).
  - If you want cookie-based API auth directly, update middleware to read from cookie and bearer with explicit precedence and tests.

### 2) Secure cookie on HTTP localhost defaults breaks local OAuth (High)

- **Where**
  - `set_secure(true)` in `backend/src/features/auth/discord/handler.rs`
  - Defaults are HTTP (`FRONTEND_URL` and `DISCORD_REDIRECT_URL`) in `backend/src/config.rs`
- **Impact**
  - Browser typically will not send/set secure cookies over plain HTTP localhost.
  - OAuth flow fails in local dev unless HTTPS is configured.
- **Ideal mitigation**
  - Add environment-driven cookie policy:
    - `COOKIE_SECURE=true` in production
    - `COOKIE_SECURE=false` for local HTTP development only
  - Keep production defaults strict and explicit.

### 3) Request-time `unwrap()` in OAuth client setup can panic server (High)

- **Where**
  - OAuth URL/client construction in `backend/src/features/auth/discord/handler.rs`
- **Impact**
  - Bad config can trigger runtime panic instead of controlled startup/config error.
- **Ideal mitigation**
  - Validate and parse OAuth URLs at startup in config loading.
  - Store typed/validated values in app config/state.
  - Replace `unwrap()` with structured error propagation.

### 4) Unique constraint violations in `PUT /users/me` map to 500 instead of 409 (Medium)

- **Where**
  - `backend/src/features/users/handler.rs` update path
  - Generic DB error mapping in `backend/src/error.rs`
- **Impact**
  - User-facing conflicts (username/email already used) are reported as server errors.
- **Ideal mitigation**
  - Add conflict mapping for user updates (similar to auth register conflict handling).
  - Return stable conflict payloads (`409`, machine-readable code + message).

### 5) Error responses expose internal DB/details (Medium)

- **Where**
  - `backend/src/error.rs` returns raw `DbErr`/internal messages
- **Impact**
  - Potential information leakage about schema/internals.
- **Ideal mitigation**
  - Return safe public error messages.
  - Keep detailed cause in structured logs with request correlation ID.

### 6) Cross-slice handler DTO dependency (Medium, architecture hygiene)

- **Where**
  - `backend/src/features/auth/local/handler.rs` imports `users::handler::UserResponse`
- **Impact**
  - Handler-layer coupling across slices weakens slice boundaries.
- **Ideal mitigation (pragmatic slice style)**
  - Move shared DTO into a lightweight shared contract module (for example `features/users/dto.rs` or `features/shared/user_contract.rs`).
  - Keep direct handler-to-handler imports discouraged.

### 7) API token hashing tied to JWT secret couples secret rotation domains (Medium)

- **Where**
  - Hashing in `backend/src/features/api_tokens/mod.rs` and use sites
- **Impact**
  - Rotating JWT signing key also invalidates API tokens unintentionally.
- **Ideal mitigation**
  - Introduce a dedicated `API_TOKEN_PEPPER` (or key derivation namespace) separate from JWT signing secret.
  - Support staged rotation with key versioning if needed.

## Recommended Auth Model for Your Product Direction

Given your constraints (Discord-heavy users, future native clients, possible BFF), the most durable model is:

### Identity vs Session/Token separation

- **Identity providers**: Discord OAuth and local admin login are both upstream identity methods.
- **First-party auth contract**: your backend always issues your own access/refresh credentials after identity is verified.

### Suggested canonical contract

- **API**: bearer access token (`Authorization: Bearer <access_token>`) as the canonical API auth mechanism.
- **Refresh**: refresh token rotation endpoint.
- **Discord/local**: both end by minting the same first-party token pair.

### Client-specific delivery

- **Web SPA (without BFF)**
  - Access token short-lived, held in memory.
  - Refresh token in HTTP-only secure cookie.
  - Silent refresh endpoint to renew access token.
- **Web with BFF (optional future)**
  - Browser holds only BFF session cookie.
  - BFF handles token lifecycle and calls API with bearer tokens server-side.
- **Native/mobile/CLI**
  - OAuth (PKCE where applicable) + token exchange to first-party token pair.
  - Store refresh token in platform secure storage.

This gives one backend auth core while letting web/native/BFF vary only at the edge.

## Why this fits your user mix

- Discord being primary works naturally as one identity provider.
- Admin/root can remain local credentials with stricter policy (MFA later) while sharing the same downstream token/session model.
- You avoid bifurcated security logic (cookie-only for one user type, bearer-only for another).

## Pragmatic Slice Guidance

For pragmatic vertical slices, keep these boundaries:

- Feature owns handlers/routes/domain behavior (`auth`, `users`, `api_tokens`).
- Shared infra modules stay small (`config`, `db`, `error`, `middleware`).
- Shared DTOs/contracts are allowed, but keep them out of handler modules.
- Cross-slice calls should be through explicit service functions/contracts, not handler imports.

## Phased Mitigation Plan

1. **Auth contract decision and enforcement**
   - Choose canonical API auth (recommended: bearer).
   - Make middleware and frontend/backend flows consistent.
2. **Cookie/runtime hardening**
   - Add `COOKIE_SECURE` env policy.
   - Remove request-time `unwrap()` from OAuth path.
3. **Error handling quality**
   - Add conflict mapping for user updates.
   - Sanitize API error payloads.
4. **Secret domain separation**
   - Add `API_TOKEN_PEPPER` and migration/compat strategy.
5. **Slice hygiene cleanup**
   - Move shared user DTO out of handler module.
6. **Coverage improvements**
   - Add integration tests for Discord callback -> authenticated `GET /users/me`.
   - Add tests for username/email conflict paths on update.

## Testing Gaps to Close

Current unit tests pass, but there is no end-to-end auth contract test for cookie vs bearer behavior. Add:

- Discord callback success followed by authenticated request test
- Local login/register followed by authenticated request test
- Cookie security behavior test by environment profile
- Conflict mapping tests for `PUT /users/me`

## Decision Recommendation (Short Version)

If you want the least long-term friction across web + native:

- Standardize backend API on bearer access tokens.
- Treat Discord/local as identity providers only.
- Use refresh-token rotation and short-lived access tokens.
- Optionally add BFF later without changing core auth internals.

