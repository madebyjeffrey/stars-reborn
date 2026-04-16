# Frontend Auth Fixes - Quick Testing Guide

## ✅ What's Fixed

All 5 critical issues from the compatibility report have been resolved:

1. ✅ **Token Storage** - Now in-memory only (not localStorage)
2. ✅ **Token Refresh** - Automatic on 401 via new `/api/auth/refresh` endpoint
3. ✅ **Discord Callback** - Now uses proper refresh flow
4. ✅ **withCredentials** - All requests include credentials for HTTP-only cookies
5. ✅ **Error Handling** - Supports new error code format

---

## 🧪 Test Steps

### Test 1: Local Login
```
1. Navigate to /auth/login
2. Enter username and password
3. Click "Login"
✓ Should redirect to /dashboard
✓ No localStorage tokens in DevTools
✓ Network tab shows Authorization header
```

### Test 2: Local Register
```
1. Navigate to /auth/register
2. Fill in username, email (optional), password
3. Click "Register"
✓ Should redirect to /dashboard
✓ Access token stored in memory, not localStorage
✓ Refresh token in cookie (HttpOnly, Secure flags)
```

### Test 3: Error Messages
```
1. Try to register with existing username
✓ Should show "Username already taken" (not DB error)

2. Try to register with existing email
✓ Should show "Email already in use"

3. Try to login with wrong password
✓ Should show "Invalid username or password"
```

### Test 4: Discord Login
```
1. Click "Login with Discord"
2. Complete Discord OAuth flow
3. Get redirected to callback
✓ Should call /api/auth/refresh automatically
✓ Should get access token and fetch user
✓ Should redirect to /dashboard
```

### Test 5: Token Refresh
```
1. Login successfully
2. Open DevTools → Storage
3. Check cookies - should see refresh_token (HttpOnly)
4. Check no auth_token in localStorage
5. Wait for access token to expire (or manually test):
   - Make any API call
   - Access token expires (401)
   ✓ Interceptor catches 401
   ✓ Calls /api/auth/refresh
   ✓ Retries original request
   ✓ Request succeeds with new token
```

### Test 6: Logout
```
1. Login successfully
2. Click Logout button
3. Should call POST /api/auth/logout
✓ Should redirect to /auth/login
✓ Local state cleared
✓ Access token cleared
✓ Next API call requires fresh login
```

### Test 7: Cross-Domain (if applicable)
```
1. Frontend on http://localhost:4200
2. Backend on http://localhost:3000
3. Login and make API call
✓ Should work (withCredentials: true)
✓ Cookies sent despite different origin
```

---

## 🔍 What to Check in DevTools

### Network Tab
✓ Login request: POST `/api/auth/login`
  - Status: 200 OK
  - Response includes `access_token`
  - Response header: `Set-Cookie: refresh_token=...`

✓ API requests: Should have header
  - `Authorization: Bearer <token>`

✓ Refresh request: POST `/api/auth/refresh`
  - Status: 200 OK
  - Cookie sent: `refresh_token` visible in request
  - Response: `{ "access_token": "..." }`

### Application Tab (Storage)
✓ **Cookies**
  - `refresh_token` should exist (HttpOnly, Secure, SameSite=Lax)
  - Should NOT see `auth_token`

✓ **Local Storage**
  - Should be EMPTY (no auth_token)

✓ **Session Storage**
  - Should be EMPTY

### Console Tab
✓ Should not see errors related to:
  - Token not found
  - Missing authorization header
  - Cookie access denied

---

## 📊 Debugging Checklist

### If Login Fails
- [ ] Check backend is running on correct port
- [ ] Check environment.apiUrl is correct in `environment.ts`
- [ ] Check error message (should be specific code, not DB error)
- [ ] Check Network tab for request/response

### If Token Doesn't Get Stored
- [ ] Check Network response includes `access_token` field
- [ ] Check response format: `{ "token": "...", "user": {...} }`
- [ ] Check auth service `handleAuth()` is called

### If Refresh Fails
- [ ] Check backend `/api/auth/refresh` endpoint exists
- [ ] Check refresh request includes `refresh_token` cookie
- [ ] Check cookie is HttpOnly (not JavaScript-accessible)
- [ ] Check Network tab for 401 → refresh → retry sequence

### If Discord Login Doesn't Work
- [ ] Check `handleDiscordCallback()` is called
- [ ] Check refresh token exists in cookie
- [ ] Check `/api/auth/refresh` is called automatically
- [ ] Check Network tab for sequence: callback → refresh → fetch user → redirect

### If API Calls Get 401
- [ ] Check access token not null/undefined
- [ ] Check Authorization header is set
- [ ] Check token format is `Bearer <token>` (not just `<token>`)
- [ ] Check interceptor is catching 401 and calling refresh

---

## 🚀 Deployment Checklist

Before deploying to production:

- [ ] All tests pass (npm test)
- [ ] No console errors
- [ ] Login/register flow works
- [ ] Discord flow works
- [ ] Token refresh automatic on 401
- [ ] Logout works and clears state
- [ ] Error messages are user-friendly
- [ ] No localStorage tokens visible
- [ ] Cookies sent on cross-origin requests
- [ ] Production backend URL in environment.prod.ts

---

## 📝 Known Behavior Changes

### Before
- Access token stored in localStorage
- Manual logout (no backend call)
- No automatic token refresh on 401
- Discord login relied on cookie-based auth

### After
- Access token in-memory only (cleared on page reload)
- Logout calls backend to revoke session
- Automatic token refresh on 401
- Discord login uses refresh endpoint
- Users stay logged in via refresh token (30 days)

### For End Users
- Tokens don't persist across page reloads (need to login again)
- Tokens refresh automatically (no interruption on 401)
- Logout properly revokes sessions
- Error messages are clearer

---

## ❓ FAQ

**Q: Where are tokens stored?**
A: Access tokens in JavaScript memory (cleared on reload). Refresh tokens in HTTP-only cookie (never accessible to JavaScript).

**Q: What happens if I refresh the page?**
A: Access token is lost (reload required). Refresh token remains in cookie, so you can login again with single API call.

**Q: How long do tokens last?**
A: Access: 7 days. Refresh: 30 days (renewed on use).

**Q: Why does interceptor retry on 401?**
A: To handle token expiration gracefully without requiring user to re-login.

**Q: Is this secure?**
A: Yes - tokens in HTTP-only cookies can't be accessed by malicious JavaScript, access tokens are short-lived, and refresh tokens are rotated.

**Q: What if refresh fails?**
A: User is logged out automatically. Next request will redirect to login page.


