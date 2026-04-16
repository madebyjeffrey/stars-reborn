import { HttpInterceptorFn } from '@angular/common/http';
import { inject } from '@angular/core';
import { catchError, switchMap, throwError } from 'rxjs';
import { AuthService } from './auth.service';

export const authInterceptor: HttpInterceptorFn = (req, next) => {
  const authService = inject(AuthService);

  // Always include credentials (needed for refresh cookie to be sent)
  req = req.clone({ withCredentials: true });

  // Add access token to Authorization header if available
  const token = authService.getAccessToken();
  if (token) {
    req = req.clone({
      headers: req.headers.set('Authorization', `Bearer ${token}`)
    });
  }

  return next(req).pipe(
    catchError(error => {
      // If we get a 401 (Unauthorized), try to refresh the access token
      if (error.status === 401) {
        // Don't try to refresh for the refresh endpoint itself (avoid infinite loops)
        if (req.url.includes('/auth/refresh') || req.url.includes('/auth/logout')) {
          return throwError(() => error);
        }

        // Attempt to refresh the access token
        return authService.refreshAccessToken().pipe(
          switchMap(res => {
            // Update the access token in the service
            authService['setAccessToken'](res.access_token);

            // Clone the original request with the new token
            const newReq = req.clone({
              headers: req.headers.set('Authorization', `Bearer ${res.access_token}`)
            });

            // Retry the original request with the new token
            return next(newReq);
          }),
          catchError(() => {
            // Refresh failed, logout user
            authService.logout();
            return throwError(() => error);
          })
        );
      }

      // For other errors, pass through
      return throwError(() => error);
    })
  );
};


