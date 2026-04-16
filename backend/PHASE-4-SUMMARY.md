# Phase 4 Implementation Summary - Error Quality & Conflict Handling

Implemented: 2026-04-16

## Overview

Phase 4 improves API error handling by sanitizing responses, adding structured error codes, and properly mapping unique constraint violations to 409 Conflict responses.

## Changes Made

### 1. Structured Error Codes (`backend/src/error.rs`)

Added `ErrorCode` enum with safe, machine-readable error codes:
- `UNAUTHORIZED` - auth failure
- `INVALID_CREDENTIALS` - login failed
- `TOKEN_EXPIRED` - JWT expired
- `NOT_FOUND` - resource not found
- `BAD_REQUEST` - validation error
- `CONFLICT_USERNAME` - username already in use
- `CONFLICT_EMAIL` - email already in use
- `INTERNAL_SERVER_ERROR` - generic server error

### 2. Public Error Message Sanitization

Updated `AppError::IntoResponse()`:
- **Database errors**: Raw DB errors logged (not exposed). Client receives generic "An error occurred" message
- **Internal errors**: Stack traces logged. Client receives generic message
- **Auth errors**: Safe to expose (e.g., "Invalid username or password")
- **NotFound/BadRequest/Conflict**: Descriptive messages exposed (user-facing)
- **Response format**: `{ "error": "<message>", "code": "<ERROR_CODE>" }`

### 3. User Update Conflict Mapping (`backend/src/features/users/handler.rs`)

Added `map_user_unique_constraint_error()` helper:
- Detects username unique constraint violations → `AppError::Conflict("Username already taken")`
- Detects email unique constraint violations → `AppError::Conflict("Email already in use")`
- Other DB errors pass through as `AppError::Database`

Applied to `PUT /api/users/me` update handler:
- Now returns `409 Conflict` with appropriate error code instead of `500 Internal Server Error`
- Username and email updates validate against existing data
- Error codes distinguish between username/email conflicts

### 4. Error Response Examples

**Before (leaked details):**
```json
{
  "error": "duplicate key value violates unique constraint \"users_username_key\""
}
```
HTTP Status: 500

**After (safe public response):**
```json
{
  "error": "Username already taken",
  "code": "CONFLICT_USERNAME"
}
```
HTTP Status: 409

**Internal server error before (leaked details):**
```json
{
  "error": "failed to execute query: connection refused"
}
```
HTTP Status: 500

**Internal server error after (sanitized):**
```json
{
  "error": "An error occurred while processing your request",
  "code": "INTERNAL_SERVER_ERROR"
}
```
HTTP Status: 500

## Test Coverage

Added 11 new tests:

**Error sanitization tests (6):**
- Database errors don't leak details
- Auth errors are exposed (safe)
- Conflict errors get correct codes
- Internal errors are sanitized
- Unauthorized errors work correctly

**Conflict mapping tests (4):**
- Username conflict detection
- Email conflict detection
- Case-insensitive detection
- Generic DB errors pass through

**Existing tests still passing (36):**
- Config parsing
- JWT operations
- Auth flows
- User operations

**Total: 46 tests passing**

## Tracing/Logging

All error details are now logged via `tracing::error!()`:
- Database errors logged with full details
- Internal errors logged with stack traces
- No sensitive data in HTTP responses
- Server operators can debug via logs while users see safe messages

## API Contract Update

### Error Response Format
All error responses now include `code` field for client-side handling:

```json
{
  "error": "Human-readable message",
  "code": "MACHINE_READABLE_CODE"
}
```

Clients can:
1. Display human-readable message to user
2. Handle specific error codes programmatically (e.g., retry on `TOKEN_EXPIRED`, show form validation on `CONFLICT_*`)

## Security Improvements

✅ No DB schema details exposed (prevents reconnaissance)
✅ No stack traces in responses (prevents information leakage)
✅ No internal state exposed (e.g., connection strings, query details)
✅ All sensitive errors logged server-side with tracing
✅ Client receives only actionable, safe information

## Backward Compatibility

- Error response format changed (added `code` field)
- HTTP status codes now correct (409 instead of 500 for conflicts)
- This is a **breaking change** for clients expecting old format
- Migration: clients should:
  - Add optional handling for new `code` field
  - Update error display logic (errors now return 409 instead of 500)

## Next Steps: Phase 5

Decouple API token hashing from JWT secret rotation by introducing `API_TOKEN_PEPPER` configuration.

