# Frontend Auth Fixes - Implementation Details

## Overview of Changes

This document provides detailed technical information about the frontend authentication fixes implemented to be compatible with backend Phases 2-5.

---

## 1. Auth Service (`auth.service.ts`)

### Key Changes

#### Removed
- `TOKEN_KEY` constant (no more localStorage)
- `getToken()` method (was returning localStorage)
- Comments about "HTTP-only cookie flow"

#### Added
```typescript
// In-memory token storage (never persisted)
private accessToken: string | null = null;
private refreshTokenExpiresAt: number | null = null;

// Token management
setAccessToken(token: string, expiresIn?: number): void
clearAccessToken(): void

// New endpoint for refresh
refreshAccessToken(): Observable<RefreshResponse>

// Updated Discord flow
handleDiscordCallback(): void

// New logout implementation
logout(): void
```

#### Modified Methods

**login()**
```typescript
// Before
return this.http.post<AuthResponse>(`${environment.apiUrl}/auth/login`, ...)

// After
return this.http.post<AuthResponse>(..., {
  withCredentials: true  // ← Added
})
```

**register()**
- Same change as login: added `withCredentials: true`

**handleAuth()**
```typescript
// Before
localStorage.setItem(this.TOKEN_KEY, res.token);

// After
this.setAccessToken(res.token);  // ← In-memory only
```

**handleDiscordCallback()**
```typescript
// Before
this.fetchCurrentUser().subscribe({
  next: () => {
    localStorage.setItem(this.TOKEN_KEY, 'authenticated');  // ← Hack
    this.router.navigate(['/dashboard']);
  }
});

// After
this.refreshAccessToken().subscribe({
  next: (res) => {
    this.setAccessToken(res.access_token);  // ← Proper flow
    this.fetchCurrentUser().subscribe({
      next: () => this.router.navigate(['/dashboard']),
      error: () => this.clearAuth()
    });
  },
  error: () => this.clearAuth()
});
```

**logout()**
```typescript
// Before
localStorage.removeItem(this.TOKEN_KEY);
this.currentUser.set(null);
this.router.navigate(['/auth/login']);

// After
this.http.post(`${environment.apiUrl}/auth/logout`, {}, { withCredentials: true })
  .subscribe({
    complete: () => this.clearAuth(),  // ← Calls backend first
    error: () => this.clearAuth()     // ← Clear state even if endpoint fails
  });
```

**isAuthenticated()**
```typescript
// Before
return !!this.currentUser();

// After
return !!this.currentUser() && !!this.accessToken;  // ← Check both
```

---

## 2. Auth Interceptor (`auth.interceptor.ts`)

### Complete Rewrite

#### Key Addition: 401 Handling

```typescript
return next(req).pipe(
  catchError(error => {
    if (error.status === 401) {
      // Skip refresh for auth endpoints (infinite loop prevention)
      if (req.url.includes('/auth/refresh') || req.url.includes('/auth/logout')) {
        return throwError(() => error);
      }

      // Attempt token refresh
      return authService.refreshAccessToken().pipe(
        switchMap(res => {
          // Update service with new token
          authService['setAccessToken'](res.access_token);  // ← Access private method

          // Clone request with new token
          const newReq = req.clone({
            headers: req.headers.set('Authorization', `Bearer ${res.access_token}`)
          });

          // Retry original request
          return next(newReq);
        }),
        catchError(() => {
          // Refresh failed, logout
          authService.logout();
          return throwError(() => error);
        })
      );
    }

    // Pass through other errors
    return throwError(() => error);
  })
);
```

#### Request Enhancement

```typescript
// Always include credentials
req = req.clone({ withCredentials: true });

// Add token if available
const token = authService.getAccessToken();
if (token) {
  req = req.clone({
    headers: req.headers.set('Authorization', `Bearer ${token}`)
  });
}
```

### Flow Diagram

```
HTTP Request
    ↓
[Add withCredentials: true]
    ↓
[Add Authorization: Bearer <token>]
    ↓
Send Request
    ↓
Response ──success──→ Return Response
    ↓
    ├─ error (401)
    │    ↓
    │  [Is auth endpoint?] ──yes──→ Throw Error
    │    ↓ no
    │  [Call /api/auth/refresh] ──success──→ [Retry original request with new token]
    │    ↓
    │  [Refresh failed] → [Logout user] → [Throw error]
    ↓
[Other error] → [Pass through]
```

---

## 3. Component Updates

### Login Component (`login.component.ts`)

Added error code interpretation:

```typescript
private getErrorMessage(err: any): string {
  if (err.error?.code) {
    switch (err.error.code) {
      case 'INVALID_CREDENTIALS':
        return 'Invalid username or password';
      case 'CONFLICT_USERNAME':
        return 'Username already taken';
      case 'CONFLICT_EMAIL':
        return 'Email already in use';
      case 'TOKEN_EXPIRED':
        return 'Your session has expired, please login again';
      case 'UNAUTHORIZED':
        return 'Authentication failed';
      default:
        return err.error?.error || 'Login failed';
    }
  }
  return err.error?.error || 'Login failed';
}
```

### Register Component (`register.component.ts`)

Similar error handling adapted for registration errors.

### Discord Callback Component

Minor comment updates - actual logic already delegated to `authService.handleDiscordCallback()`.

---

## 4. API Contract Alignment

### Before (Incompatible)

Frontend expected:
```
POST /api/auth/login
→ Cookie-based auth via middleware
→ No refresh mechanism
→ localStorage storage
```

### After (Compatible)

Frontend now uses:
```
POST /api/auth/login
Response: 
  - Set-Cookie: refresh_token=<jwt>
  - { "token": "<access>", "user": {...} }
  ↓
Store access_token in memory
Store refresh_token in cookie (automatic)
  ↓
Use access_token for API calls
  ↓
On 401: Call POST /api/auth/refresh (with cookie)
  ↓
Get new access_token, retry request
```

---

## 5. Security Analysis

### Token Storage

| Location | Security | Persistence | Access |
|----------|----------|-------------|--------|
| localStorage (OLD) | ❌ XSS vulnerable | ✅ Survives reload | JavaScript |
| Memory (NEW) | ✅ Safe from XSS | ❌ Lost on reload | JavaScript |
| HTTP-only Cookie | ✅ Safe from XSS | ✅ Survives reload | Browser/Server only |

### Flow Security

**Before:**
- Tokens in localStorage → XSS exposure
- No refresh → expired tokens require re-login
- No proper logout → sessions linger

**After:**
- Access token in memory → XSS safe
- Refresh token in HTTP-only cookie → XSS safe
- Auto-refresh on 401 → seamless user experience
- Backend logout → properly revokes sessions
- Token rotation → replay attack detection

---

## 6. State Management

### Initialization

```
App Load
  ↓
AuthService constructor
  ↓
loadUser()
  ↓
[Call /api/users/me]
  ├─ Success: Set currentUser (user was logged in)
  └─ Error: Clear auth (not logged in)
```

### Login State

```
Before: localStorage["auth_token"] + currentUser signal
After:  accessToken (memory) + currentUser signal + refreshTokenExpiresAt
```

### Auth Flow States

```
Not Logged In
  ↓ [User submits login]
  ↓
Logging In (loading=true)
  ↓ [Backend returns access_token + sets refresh_token cookie]
  ↓
Logged In (accessToken != null && currentUser != null)
  ↓ [Access token expires in 7 days]
  ↓
[Interceptor catches 401]
  ↓ [Calls /api/auth/refresh]
  ↓
Refreshing Token
  ↓ [Backend returns new access_token]
  ↓
Logged In (new token active)
  ↓ [User clicks logout]
  ↓
Logging Out
  ↓ [Backend revokes session]
  ↓
Not Logged In
```

---

## 7. Error Handling

### Backend Error Responses

New format (Phase 4):
```json
{
  "error": "Username already taken",
  "code": "CONFLICT_USERNAME"
}
```

Old format (not supported):
```json
{
  "error": "duplicate key value violates unique constraint \"users_username_key\""
}
```

### Frontend Error Mapping

```typescript
// Maps error codes to user messages
Error Code → User Message
CONFLICT_USERNAME → "Username already taken"
CONFLICT_EMAIL → "Email already in use"
INVALID_CREDENTIALS → "Invalid username or password"
TOKEN_EXPIRED → "Your session has expired, please login again"
UNAUTHORIZED → "Authentication failed"
(unknown) → (use raw error message)
```

---

## 8. HTTP Request/Response Examples

### Login Request
```http
POST /api/auth/login HTTP/1.1
Host: api.example.com
Content-Type: application/json
Cookie: (none initially)

{
  "username": "user",
  "password": "password"
}
```

### Login Response
```http
HTTP/1.1 200 OK
Set-Cookie: refresh_token=eyJ...; HttpOnly; Secure; SameSite=Lax; Max-Age=2592000; Path=/

{
  "token": "eyJ...",
  "user": {
    "id": "uuid",
    "username": "user",
    "email": "user@example.com"
  }
}
```

### API Request (with token)
```http
GET /api/users/me HTTP/1.1
Host: api.example.com
Authorization: Bearer eyJ...
Cookie: refresh_token=eyJ...
```

### Token Refresh on 401
```http
GET /api/users/me HTTP/1.1
→ 401 Unauthorized

POST /api/auth/refresh HTTP/1.1
Authorization: Bearer eyJ...
Cookie: refresh_token=eyJ...

Response:
{
  "access_token": "eyJ..."
}

GET /api/users/me HTTP/1.1  [Retry with new token]
Authorization: Bearer eyJ...[NEW]
```

---

## 9. Testing Strategy

### Unit Tests
- Token storage/retrieval
- Error message mapping
- Request/response handling

### Integration Tests
- Full login flow
- Token refresh flow
- Logout flow
- Discord callback flow
- Cross-domain (if applicable)

### E2E Tests
- Login → API call → Logout
- Login → Wait for expiration → API call (triggers refresh)
- Discord login → Refresh → API call

---

## 10. Migration from Old Implementation

### For Existing Users

Session before fix:
```
localStorage["auth_token"] = "old_token"
```

After upgrade:
```
// Old token ignored
accessToken = null (memory)
refreshToken = null (cookie)
currentUser = null

User sees: Not logged in (required to login again)
```

### For New Users

Works with new implementation immediately.

### Compatibility Window

Not needed - complete rewrite. Old format tokens not accepted by backend anyway (Phase 3 changed contract).

---

## 11. Browser DevTools Inspection

### Network Tab
Should see:
- `POST /api/auth/login` → 200 with Set-Cookie header
- `GET /api/users/me` → 200 with `Authorization: Bearer ...` header
- `POST /api/auth/refresh` → 200 with cookie sent

### Application Tab - Cookies
Should see:
- `refresh_token` (HttpOnly, Secure, SameSite=Lax)
- No `auth_token`

### Application Tab - Local Storage
Should be empty.

### Console
Should have no errors related to:
- Token not found
- Missing Authorization header
- Cookie access

---

## 12. Performance Considerations

### Token Refresh Overhead
- Minimal - only on 401 (token expiration)
- Access token TTL: 7 days
- Refresh token TTL: 30 days

### Request Overhead
- `withCredentials: true` minimal impact
- Cookies sent on every request (standard web behavior)
- Refresh endpoint is quick (<10ms expected)

### Memory Usage
- One string in memory (access token) - minimal
- One signal (currentUser) - negligible
- One timestamp (refreshTokenExpiresAt) - negligible

---

## 13. Debugging Tips

### Check Token State
```typescript
// In browser console
localStorage  // Should be empty
sessionStorage  // Should be empty
document.cookie  // Should show refresh_token

// Angular DevTools or direct
authService['accessToken']  // Should have token
authService['refreshTokenExpiresAt']  // Should have timestamp
```

### Monitor Requests
1. Open DevTools → Network tab
2. Filter by XHR/Fetch
3. Look for:
   - POST /api/auth/login → should set refresh_token cookie
   - GET /api/users/me → should have Authorization header
   - Any 401 → should trigger POST /api/auth/refresh

### Trace Auth Flow
```typescript
// Add logging to auth.service.ts
setAccessToken(token: string, expiresIn?: number): void {
  console.log('Setting access token', { token: token.substring(0, 10) + '...' });
  this.accessToken = token;
  // ...
}
```

---

## Conclusion

The frontend has been completely refactored to implement:
1. ✅ Secure in-memory token storage
2. ✅ Automatic token refresh on 401
3. ✅ Proper logout with session revocation
4. ✅ HTTP-only cookie credential handling
5. ✅ New error code interpretation

All changes follow Angular best practices and web security standards.


