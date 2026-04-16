# Frontend-Backend Auth Compatibility Report
**Date: April 16, 2026**

## Executive Summary

⚠️ **COMPATIBILITY STATUS: PARTIAL** - The frontend is **NOT fully compatible** with the recent backend auth changes (Phases 2-5). Critical issues exist in how the frontend handles refresh tokens and access tokens after the backend implemented a unified access+refresh token architecture.

**Key Finding**: The backend now uses a new refresh token rotation model with HTTP-only cookies, but the frontend is still using deprecated patterns and does not properly implement the required token refresh flow.

---

## Backend Auth Changes Summary (Phases 2-5)

### What Changed
1. **Phase 2 (Token Foundation)**: 
   - Introduced typed JWT claims (`typ: Access` vs `typ: Refresh`)
   - Added `jti` (JWT ID) for session tracking
   - Created `refresh_sessions` table for refresh token management
   - New endpoints: `POST /api/auth/refresh`, `POST /api/auth/logout`

2. **Phase 3 (Unified Contract)**:
   - Both Discord and local login now return **access token in response body** and **refresh token in HTTP-only cookie**
   - Login/register responses: `{ "token": "<access_token>", "user": {...} }`
   - Refresh token stored in `refresh_token` HTTP-only cookie (7-day access, 30-day refresh)

3. **Phase 4 (Error Quality)**:
   - Error responses now include machine-readable `code` field
   - Unique constraint violations return `409` instead of `500`
   - Database details are sanitized from public error messages

4. **Phase 5 (API Token Security)**:
   - API tokens now use separate `API_TOKEN_PEPPER` (not coupled to JWT secret)
   - Bearer token authentication remains unchanged

### New API Contract
```
POST /api/auth/login
Request:  { "username": "...", "password": "..." }
Response: 
  Set-Cookie: refresh_token=<jwt>; HttpOnly; Secure; SameSite=Lax; Max-Age=2592000; Path=/
  {
    "token": "<access_token>",
    "user": { "id": "...", "username": "...", ... }
  }

POST /api/auth/refresh
Request:  (cookie auto-sent)
Response: { "access_token": "..." }

POST /api/auth/logout
Request:  Authorization: Bearer <token>
Response: { "message": "Logged out successfully" }
```

---

## Frontend Current Implementation

### How It Currently Works
1. **Auth Service** (`auth.service.ts`):
   - Stores token in `localStorage` with key `auth_token`
   - `getToken()` retrieves from localStorage
   - `handleAuth(res)` stores `res.token` in localStorage
   - Comments acknowledge HTTP-only cookies exist but don't properly use them

2. **Auth Interceptor** (`auth.interceptor.ts`):
   - Checks if token exists and isn't the string `"authenticated"`
   - Sets `Authorization: Bearer <token>` header
   - Has special case for OAuth flow (expects cookie-based auth)

3. **Discord Callback** (`discord-callback.component.ts`):
   - Calls `handleDiscordCallback()` which tries to fetch `/api/users/me`
   - Sets `auth_token` to string `"authenticated"` in localStorage (workaround)

4. **Login/Register Components**:
   - Call auth service methods and navigate on success
   - Display error messages from `err.error?.error`

---

## Compatibility Issues

### 🔴 Critical Issues

#### 1. **Token Storage Mismatch**
- **Frontend Does**: Stores access token in localStorage
- **Backend Expects**: Access token in memory, refresh token in HTTP-only cookie
- **Problem**: localStorage is accessible to JavaScript, defeating the security purpose of HTTP-only cookies
- **Impact**: HIGH - Violates secure token handling
- **Required Fix**: Store access token in memory only (do not use localStorage for tokens)

```typescript
// CURRENT (WRONG):
localStorage.setItem(this.TOKEN_KEY, res.token);

// SHOULD BE:
private accessToken: string | null = null;  // in-memory only
this.accessToken = res.token;
```

#### 2. **Missing Refresh Token Handling**
- **Frontend Does**: Doesn't check for expired access tokens
- **Backend Provides**: `POST /api/auth/refresh` endpoint
- **Problem**: Frontend has no mechanism to refresh tokens when they expire
- **Impact**: HIGH - Users will get 401 errors after 7 days and need to re-login
- **Required Fix**: Implement token expiration detection and automatic refresh

```typescript
// MISSING: Refresh logic when access token expires (401 response)
// MISSING: Interceptor to catch 401 and call /api/auth/refresh
```

#### 3. **Discord Callback Not Using New Flow**
- **Frontend Does**: Sets `auth_token` to `"authenticated"` string (workaround)
- **Backend Now Does**: 
  - Sets `refresh_token` cookie automatically
  - Expects frontend to call `POST /api/auth/refresh` to get access token
- **Problem**: Frontend uses outdated flow that relied on cookie-based auth
- **Impact**: MEDIUM-HIGH - Discord login likely broken
- **Required Fix**: Call `/api/auth/refresh` after Discord callback to get access token

```typescript
// CURRENT:
handleDiscordCallback(): void {
  this.fetchCurrentUser().subscribe({
    next: () => {
      localStorage.setItem(this.TOKEN_KEY, 'authenticated');  // WRONG
      this.router.navigate(['/dashboard']);
    }
  });
}

// SHOULD BE:
handleDiscordCallback(): void {
  this.refreshAccessToken().subscribe({
    next: (response) => {
      this.accessToken = response.access_token;  // Store in memory
      this.fetchCurrentUser().subscribe({...});
    }
  });
}
```

#### 4. **No Error Code Handling**
- **Frontend Does**: Only checks for `err.error?.error` field
- **Backend Now Provides**: Both `error` and `code` fields
- **Problem**: Frontend can't distinguish between different error types programmatically
- **Impact**: MEDIUM - Error handling is less robust but still somewhat functional
- **Required Fix**: Use `code` field for specific error handling

```typescript
// CURRENT:
error: (err) => {
  this.error = err.error?.error || 'Login failed';
}

// SHOULD BE:
error: (err) => {
  if (err.error?.code === 'CONFLICT_USERNAME') {
    this.error = 'Username already taken';
  } else if (err.error?.code === 'INVALID_CREDENTIALS') {
    this.error = 'Invalid username or password';
  } else {
    this.error = err.error?.error || 'Login failed';
  }
}
```

#### 5. **HTTP-Only Cookie Not Being Used for Refresh**
- **Frontend Does**: Doesn't configure HttpClient to send credentials (withCredentials)
- **Backend Expects**: Refresh endpoint to receive cookie automatically
- **Problem**: Browser won't send cookies with cross-origin requests unless `withCredentials` is set
- **Impact**: HIGH - Refresh endpoint will fail if frontend is on different domain
- **Required Fix**: Configure HttpClient interceptor to include credentials

```typescript
// MISSING: withCredentials in HttpClient requests
// SHOULD ADD to interceptor or HttpClient config:
if (req.url.includes('/api/auth/')) {
  req = req.clone({
    withCredentials: true  // Allow credentials (cookies) in request
  });
}
```

---

### 🟡 Medium Issues

#### 6. **getToken() Still Returns Stored Token**
- **Issue**: `getToken()` method returns token from localStorage, but interceptor checks this
- **Problem**: Creates confusion about where tokens should be stored
- **Impact**: MEDIUM - Code maintainability, potential logic errors
- **Required Fix**: Rename or repurpose to reflect in-memory storage

```typescript
// CURRENT:
getToken(): string | null {
  return localStorage.getItem(this.TOKEN_KEY) || null;
}

// SHOULD BE:
getAccessToken(): string | null {
  return this.accessToken;  // Return in-memory token
}
```

#### 7. **isAuthenticated() Logic Unclear**
- **Current Logic**: Checks if `currentUser` is set
- **Issue**: Doesn't verify if access token is still valid
- **Impact**: MEDIUM - May show user as authenticated after token expiration
- **Required Fix**: Also check access token validity

#### 8. **No logout Implementation**
- **Frontend Does**: Clears localStorage and sets user to null
- **Backend Now Expects**: Call to `POST /api/auth/logout` to revoke refresh session
- **Problem**: Old refresh sessions remain valid in DB
- **Impact**: MEDIUM - Security issue, old sessions remain revocable but not actively used
- **Required Fix**: Call logout endpoint before clearing local state

```typescript
// CURRENT:
logout(): void {
  localStorage.removeItem(this.TOKEN_KEY);
  this.currentUser.set(null);
  this.router.navigate(['/auth/login']);
}

// SHOULD BE:
logout(): void {
  this.http.post(`${environment.apiUrl}/auth/logout`, {}).subscribe({
    complete: () => {
      this.accessToken = null;
      this.currentUser.set(null);
      this.router.navigate(['/auth/login']);
    }
  });
}
```

---

### 🟢 Minor Issues

#### 9. **Comments Mention Old Behavior**
- Multiple comments reference "HTTP-only cookie flow" as if it's new
- Should be updated to reflect actual implementation

#### 10. **No Request/Retry on 401**
- Interceptor doesn't implement retry-on-401 pattern
- After refresh, original request should be retried
- Not critical but reduces user experience

---

## Testing Gaps

The frontend has not been tested against:
- ✗ New `/api/auth/refresh` endpoint
- ✗ New error response format with `code` field
- ✗ Refresh token rotation and replay detection
- ✗ HTTP-only cookie credential handling
- ✗ 401 → refresh → retry flow
- ✗ Discord callback → refresh flow
- ✗ Token expiration after 7 days

---

## Migration Path

### Phase 1: Immediate Fixes (Critical Path)
```
1. Remove localStorage token storage
2. Add in-memory access token storage
3. Implement /api/auth/refresh call for Discord callback
4. Add withCredentials to HttpClient interceptor
5. Implement 401 → refresh → retry logic
```

### Phase 2: Enhanced Handling
```
6. Use error `code` field for specific error handling
7. Implement proper logout with backend call
8. Add token expiration detection
9. Add automatic silent refresh before token expires
```

### Phase 3: Testing & Observability
```
10. Add integration tests for new flows
11. Add console logging for auth state transitions
12. Monitor 401 and refresh success/failure rates
```

---

## Detailed Recommendations

### 1. Refactor AuthService Token Storage
```typescript
export class AuthService {
  // In-memory token storage
  private accessToken: string | null = null;
  private refreshTokenExpiresAt: number | null = null;
  
  currentUser = signal<User | null>(null);
  
  setAccessToken(token: string, expiresIn?: number): void {
    this.accessToken = token;
    if (expiresIn) {
      this.refreshTokenExpiresAt = Date.now() + (expiresIn * 1000);
    }
  }
  
  getAccessToken(): string | null {
    return this.accessToken;
  }
  
  clearAccessToken(): void {
    this.accessToken = null;
    this.refreshTokenExpiresAt = null;
  }
}
```

### 2. Add Token Refresh Logic
```typescript
private refreshAccessToken(): Observable<{ access_token: string }> {
  return this.http.post<{ access_token: string }>(
    `${environment.apiUrl}/auth/refresh`, 
    {},
    { withCredentials: true }
  );
}

private ensureTokenValid(): Observable<string> {
  if (this.accessToken && (!this.refreshTokenExpiresAt || Date.now() < this.refreshTokenExpiresAt - 60000)) {
    return of(this.accessToken);
  }
  
  return this.refreshAccessToken().pipe(
    tap(res => this.setAccessToken(res.access_token)),
    map(res => res.access_token),
    catchError(() => {
      this.logout();
      return throwError(() => new Error('Token refresh failed'));
    })
  );
}
```

### 3. Update Auth Interceptor
```typescript
export const authInterceptor: HttpInterceptorFn = (req, next) => {
  const authService = inject(AuthService);
  
  // Add credentials for all requests (needed for refresh cookie)
  req = req.clone({ withCredentials: true });
  
  // Add access token if available
  const token = authService.getAccessToken();
  if (token) {
    req = req.clone({
      headers: req.headers.set('Authorization', `Bearer ${token}`)
    });
  }
  
  return next(req).pipe(
    catchError(error => {
      if (error.status === 401) {
        // Token expired, try to refresh
        return authService.refreshAccessToken().pipe(
          switchMap(res => {
            authService.setAccessToken(res.access_token);
            const newReq = req.clone({
              headers: req.headers.set('Authorization', `Bearer ${res.access_token}`)
            });
            return next(newReq);
          }),
          catchError(() => {
            authService.logout();
            return throwError(() => error);
          })
        );
      }
      return throwError(() => error);
    })
  );
};
```

### 4. Update Discord Callback
```typescript
handleDiscordCallback(): void {
  // Backend has set refresh_token cookie, now get access token
  this.refreshAccessToken().subscribe({
    next: (res) => {
      this.setAccessToken(res.access_token);
      this.fetchCurrentUser().subscribe({
        next: () => this.router.navigate(['/dashboard']),
        error: () => this.logout()
      });
    },
    error: () => this.logout()
  });
}
```

### 5. Implement Proper Logout
```typescript
logout(): void {
  this.http.post(`${environment.apiUrl}/auth/logout`, {}, 
    { withCredentials: true }).subscribe({
    complete: () => {
      this.clearAccessToken();
      this.currentUser.set(null);
      this.router.navigate(['/auth/login']);
    },
    error: () => {
      // Logout endpoint failed, but clear local state anyway
      this.clearAccessToken();
      this.currentUser.set(null);
      this.router.navigate(['/auth/login']);
    }
  });
}
```

---

## Risk Assessment

| Component | Risk Level | Impact if Not Fixed | Timeline |
|-----------|-----------|-------------------|----------|
| Token storage | **CRITICAL** | Security vulnerability | Immediate |
| Token refresh on 401 | **CRITICAL** | Users logged out after 7 days | Immediate |
| Discord callback flow | **HIGH** | Discord login completely broken | Urgent (before release) |
| withCredentials | **HIGH** | Refresh endpoint won't work cross-domain | Urgent |
| Error code handling | **MEDIUM** | Less robust error UX | Can defer |
| Logout endpoint | **MEDIUM** | Old sessions linger (low risk) | Can defer |

---

## Conclusion

**The frontend requires significant updates** to work with the new backend auth architecture. The most critical issues are:

1. ❌ Stop storing access tokens in localStorage
2. ❌ Implement automatic token refresh on 401
3. ❌ Fix Discord callback to use new `/api/auth/refresh` endpoint
4. ❌ Add `withCredentials: true` to HTTP requests

**Estimated effort**: 4-6 hours for a full implementation  
**Priority**: CRITICAL before next deployment  
**Testing recommendation**: Add integration tests for all auth flows including Discord callback and token expiration scenarios


