# Backend Auth Implementation Plan (Phased) - 2026-04-16

## Purpose

This report turns the review findings into an execution-ready plan you can use to request implementation phase by phase.

It is optimized for:

- Discord-first user base
- Local non-Discord admin/root users
- Web now, native clients later
- Pragmatic vertical slices

---

## Target End State

- One canonical API auth contract: `Authorization: Bearer <access_token>`.
- Discord and local login are identity providers that both issue first-party credentials.
- Short-lived access token + refresh token rotation.
- Web uses secure cookie only for refresh/session continuity (not as direct API identity proof).
- Backend slices remain pragmatic, with shared DTO/service contracts where helpful.

---

## Phase Overview

1. **Phase 0 - Decisions and constraints freeze**
2. **Phase 1 - Stopgap consistency + panic hardening**
3. **Phase 2 - Token contract foundation (access + refresh)**
4. **Phase 3 - Align Discord and local flows to same contract**
5. **Phase 4 - Middleware/auth cleanup and conflict/error handling**
6. **Phase 5 - Secret domain separation for API tokens**
7. **Phase 6 - Slice hygiene refactor (pragmatic, minimal churn)**
8. **Phase 7 - Integration and security test expansion**
9. **Phase 8 - Native-client readiness + optional BFF bridge**

Each phase can be implemented and merged independently.

---

## Phase 0 - Decisions and Constraints Freeze

### Goal
Lock critical decisions before code churn.

### Decisions to confirm

- Access token TTL (recommended: 10-20 minutes).
- Refresh token TTL (recommended: 30 days, rotate on use).
- Should web keep refresh token in HTTP-only cookie? (recommended: yes)
- Should API ever accept cookie as bearer substitute? (recommended: no)
- Required logout semantics: revoke all sessions vs current session only.

### Deliverables

- ADR markdown documenting the chosen auth contract and token lifecycle.
- Environment matrix for local/staging/prod (cookie flags, CORS, HTTPS requirements).

### Definition of done

- Team agrees on auth contract and token policy.
- No implementation begins without this baseline.

---

## Phase 1 - Stopgap Consistency + Panic Hardening

### Goal
Fix immediate correctness issues with minimal schema/API disruption.

### Problems addressed

- OAuth cookie flow mismatch with middleware.
- Request-time `unwrap()` panics.
- Local dev cookie `secure` mismatch on HTTP.

### Implementation actions

- Add typed config options:
  - `COOKIE_SECURE` (bool)
  - `COOKIE_SAME_SITE` (enum/string parsed to `SameSite`)
- Replace OAuth handler `unwrap()` calls with validated config and propagated errors.
- Ensure frontend/backend flow uses one working path temporarily (either bearer-only or explicit fallback), with clear TODO markers for Phase 2.

### Likely files

- `backend/src/config.rs`
- `backend/src/features/auth/discord/handler.rs`
- `backend/src/main.rs` (if typed config/state shape changes)

### Tests

- Unit tests for config parsing (`COOKIE_SECURE`, invalid values).
- Discord handler tests for invalid/missing OAuth config (no panic).

### Definition of done

- No auth-path `unwrap()` panics remain.
- Local dev OAuth works with documented env.
- Auth behavior is explicit and documented.

---

## Phase 2 - Token Contract Foundation (Access + Refresh)

### Goal
Introduce first-party token lifecycle primitives used by both auth methods.

### Implementation actions

- Extend JWT claims to include token type and unique token ID (`jti`):
  - `typ` = `access` or `refresh`
  - `sub` = user id
- Add refresh token storage table (recommended) to support rotation/revocation:
  - `id/jti`, `user_id`, `expires_at`, `revoked_at`, `replaced_by`, `created_at`, `last_used_at`, optional `client_fingerprint`.
- Add endpoints:
  - `POST /api/auth/refresh`
  - `POST /api/auth/logout` (revoke current refresh token/session)
- Access token remains bearer for API authorization.

### Migration work

- New migration for refresh token/session records.
- Indexes on `user_id`, `jti`, active-session lookup fields.

### Likely files

- `backend/migration/src/*new_refresh_tokens_migration*.rs`
- `backend/src/features/auth/local/*`
- `backend/src/features/auth/discord/*`
- `backend/src/jwt.rs`
- `backend/src/features/*/model.rs` (new model)

### Tests

- JWT claim type validation tests.
- Refresh rotation tests (old refresh invalid after use).
- Logout revocation tests.

### Definition of done

- Access and refresh have distinct handling and validation.
- Refresh rotation works and prevents replay.

---

## Phase 3 - Align Discord and Local Flows to One Contract

### Goal
Both login methods end in the exact same token issuance behavior.

### Implementation actions

- Local login/register:
  - Return access token payload for API use.
  - Set refresh cookie (HTTP-only, secure by env, same-site by env).
- Discord callback:
  - Complete identity upsert.
  - Issue same access+refresh pair as local flow.
  - Redirect frontend without embedding sensitive token values in URL.
- Add endpoint for frontend bootstrap:
  - `GET /api/auth/session` or continue using `GET /api/users/me` with bearer access token refreshed as needed.

### Frontend contract notes

- Access token held in memory (or short-lived storage if absolutely necessary).
- Refresh via cookie-backed `POST /api/auth/refresh`.

### Definition of done

- Discord and local users authenticate through identical downstream mechanisms.
- No split-brain cookie-vs-bearer behavior remains.

---

## Phase 4 - Middleware/Auth Cleanup + Error/Conflict Quality

### Goal
Improve correctness and API behavior quality.

### Implementation actions

- Middleware should validate only access tokens for API requests.
- Add explicit error codes/messages (safe public format):
  - `UNAUTHORIZED`, `TOKEN_EXPIRED`, `CONFLICT_USERNAME`, `CONFLICT_EMAIL`, etc.
- Stop returning raw DB/internal messages to clients.
- Add conflict mapping for `PUT /api/users/me` unique violations (`409`).

### Likely files

- `backend/src/middleware/auth.rs`
- `backend/src/error.rs`
- `backend/src/features/users/handler.rs`

### Definition of done

- `PUT /users/me` conflict returns `409` with stable payload.
- Internal DB details are logged, not exposed to clients.

---

## Phase 5 - Secret Domain Separation for API Tokens

### Goal
Decouple API token hashing from JWT signing secret rotation.

### Implementation actions

- Add `API_TOKEN_PEPPER` config (required in prod).
- Change API token hash function to use `API_TOKEN_PEPPER`.
- Add compatibility/rotation strategy:
  - Option A: invalidate all old API tokens once.
  - Option B: support dual verification during migration window.

### Likely files

- `backend/src/config.rs`
- `backend/src/features/api_tokens/mod.rs`
- `backend/src/features/api_tokens/handler.rs`
- `backend/src/middleware/auth.rs`

### Definition of done

- JWT secret rotation does not invalidate API tokens.
- Migration strategy documented and tested.

---

## Phase 6 - Slice Hygiene Refactor (Pragmatic)

### Goal
Reduce cross-slice coupling without over-engineering.

### Implementation actions

- Move shared user response contract out of handler module:
  - Example: `backend/src/features/users/dto.rs`
- Keep route/handler ownership in each slice.
- Introduce small shared auth service helpers only where duplicated logic exists.

### Likely files

- `backend/src/features/users/dto.rs` (new)
- `backend/src/features/users/mod.rs`
- `backend/src/features/auth/local/handler.rs`
- `backend/src/features/users/handler.rs`

### Definition of done

- No handler-to-handler imports across slices.
- Shared contracts are explicit and minimal.

---

## Phase 7 - Integration and Security Test Expansion

### Goal
Catch auth regressions with behavior-driven tests.

### Implementation actions

- Add integration tests for:
  - local login -> bearer access works on `/api/users/me`
  - Discord callback (mocked) -> refresh cookie set -> access obtained -> `/api/users/me`
  - refresh rotation replay rejection
  - logout revocation behavior
  - `PUT /users/me` conflict returns `409`
- Add environment-profile tests for cookie flags.

### Likely files

- `backend/tests/*` (new test modules)
- `backend/tests/common/mod.rs` (helpers for auth bootstrap)

### Definition of done

- Major auth flows validated end-to-end.
- CI gates cover contract-critical behavior.

---

## Phase 8 - Native Readiness + Optional BFF Bridge

### Goal
Prepare for future clients with minimal backend redesign.

### Native readiness actions

- Document token exchange and refresh contract for native apps.
- Ensure CORS and CSRF assumptions are web-specific and do not block native clients.

### Optional BFF path

- Add BFF-specific session endpoints if adopting server-side token management later.
- Keep core API auth unchanged (bearer remains canonical service contract).

### Definition of done

- Native client auth integration can be built without backend contract changes.
- BFF can be added as an edge pattern, not a core rewrite.

---

## Suggested Implementation Request Prompts (Copy/Paste)

Use these prompts directly when you want coding to begin.

1. **Phase 1 request**
   - "Implement Phase 1 from `backend/backend-auth-implementation-phases-2026-04-16.md`: add `COOKIE_SECURE` config, remove OAuth `unwrap()` panics, and add tests. Keep API behavior otherwise unchanged."

2. **Phase 2 request**
   - "Implement Phase 2: add refresh-token storage, typed JWT claims (`typ`, `jti`), `POST /api/auth/refresh`, and rotation tests."

3. **Phase 3 request**
   - "Implement Phase 3: make Discord and local login both issue the same access+refresh contract and update route handlers accordingly."

4. **Phase 4 request**
   - "Implement Phase 4: sanitize public error payloads, map `PUT /api/users/me` unique conflicts to `409`, and add tests."

5. **Phase 5 request**
   - "Implement Phase 5: introduce `API_TOKEN_PEPPER`, decouple API token hashing from JWT secret, and include migration/compat strategy."

6. **Phase 6 request**
   - "Implement Phase 6: refactor cross-slice user DTO usage into a shared contract module with minimal code churn."

7. **Phase 7 request**
   - "Implement Phase 7: add integration tests for login, refresh rotation, logout revocation, and conflict mapping."

---

## Rollout and Risk Control

- Ship phases behind small PRs with clear migration notes.
- For token contract changes, release with short overlap window where possible.
- Add observability before hard cutovers:
  - refresh success/failure counters
  - token validation failure reasons
  - auth middleware rejection metrics

---

## Immediate Next Best Step

Start with **Phase 1** to remove high-risk behavior (panic + cookie env mismatch), then proceed to **Phase 2/3** as the core auth contract rollout.
