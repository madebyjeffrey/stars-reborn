import { Injectable, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Router } from '@angular/router';
import { Observable, tap } from 'rxjs';
import { environment } from '../../environments/environment';

export interface User {
  id: string;
  username: string;
  email?: string;
  discord_id?: string;
  discord_username?: string;
  discord_avatar?: string;
}

export interface AuthResponse {
  token: string;
  user: User;
}

export interface RefreshResponse {
  access_token: string;
}

@Injectable({ providedIn: 'root' })
export class AuthService {
  // In-memory token storage (never persisted to localStorage for security)
  private accessToken: string | null = null;
  private refreshTokenExpiresAt: number | null = null;

  currentUser = signal<User | null>(null);

  constructor(private http: HttpClient, private router: Router) {
    this.loadUser();
  }

  private loadUser(): void {
    // Try to fetch the current user; if successful, we're authenticated.
    // The refresh token is stored in HTTP-only cookie set by the backend.
    this.fetchCurrentUser().subscribe({
      error: () => this.clearAuth()
    });
  }

  getAccessToken(): string | null {
    // Return in-memory access token (never from localStorage)
    return this.accessToken;
  }

  private setAccessToken(token: string, expiresIn?: number): void {
    this.accessToken = token;
    if (expiresIn) {
      this.refreshTokenExpiresAt = Date.now() + (expiresIn * 1000);
    }
  }

  private clearAccessToken(): void {
    this.accessToken = null;
    this.refreshTokenExpiresAt = null;
  }

  isAuthenticated(): boolean {
    // Check if we have both a current user and a valid access token
    return !!this.currentUser() && !!this.accessToken;
  }

  login(username: string, password: string): Observable<AuthResponse> {
    return this.http.post<AuthResponse>(`${environment.apiUrl}/auth/login`, { username, password }, {
      withCredentials: true
    }).pipe(
      tap(res => this.handleAuth(res))
    );
  }

  register(username: string, password: string, email?: string): Observable<AuthResponse> {
    return this.http.post<AuthResponse>(`${environment.apiUrl}/auth/register`, { username, password, email }, {
      withCredentials: true
    }).pipe(
      tap(res => this.handleAuth(res))
    );
  }

  loginWithDiscord(): void {
    window.location.href = `${environment.apiUrl}/auth/discord`;
  }

  handleDiscordCallback(): void {
    // The backend has set the refresh_token in an HTTP-only cookie.
    // Now we need to get the access token by calling the refresh endpoint.
    this.refreshAccessToken().subscribe({
      next: (res) => {
        this.setAccessToken(res.access_token);
        // Fetch the current user to load user data
        this.fetchCurrentUser().subscribe({
          next: () => this.router.navigate(['/dashboard']),
          error: () => this.clearAuth()
        });
      },
      error: () => this.clearAuth()
    });
  }

  refreshAccessToken(): Observable<RefreshResponse> {
    return this.http.post<RefreshResponse>(
      `${environment.apiUrl}/auth/refresh`,
      {},
      { withCredentials: true }
    );
  }

  private handleAuth(res: AuthResponse): void {
    // Store access token in memory (never localStorage)
    // Backend also sets refresh_token in HTTP-only cookie
    this.setAccessToken(res.token);
    this.currentUser.set(res.user);
  }

  fetchCurrentUser(): Observable<User> {
    return this.http.get<User>(`${environment.apiUrl}/users/me`, {
      withCredentials: true
    }).pipe(
      tap(user => this.currentUser.set(user))
    );
  }

  logout(): void {
    // Call backend to revoke the refresh session
    this.http.post(
      `${environment.apiUrl}/auth/logout`,
      {},
      { withCredentials: true }
    ).subscribe({
      complete: () => this.clearAuth(),
      error: () => this.clearAuth()  // Clear local state even if logout fails
    });
  }

  private clearAuth(): void {
    this.clearAccessToken();
    this.currentUser.set(null);
    this.router.navigate(['/auth/login']);
  }
}
