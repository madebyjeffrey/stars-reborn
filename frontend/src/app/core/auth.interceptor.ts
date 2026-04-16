import { HttpInterceptorFn } from '@angular/common/http';
import { inject } from '@angular/core';
import { AuthService } from './auth.service';

export const authInterceptor: HttpInterceptorFn = (req, next) => {
  const authService = inject(AuthService);
  const token = authService.getToken();

  // Only add Authorization header for actual JWT tokens (traditional login/register).
  // For OAuth flow, authentication is handled via HTTP-only cookie sent automatically by the browser.
  // The token value "authenticated" is just a flag and should not be sent as a Bearer token.
  if (token && token !== 'authenticated') {
    const cloned = req.clone({
      headers: req.headers.set('Authorization', `Bearer ${token}`)
    });
    return next(cloned);
  }

  // For OAuth flow: browser automatically sends auth_token HTTP-only cookie with the request.
  // No need to manually set Authorization header.
  return next(req);
};


