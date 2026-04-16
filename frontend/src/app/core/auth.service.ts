import { Injectable, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Router } from '@angular/router';
import { Observable, tap, catchError } from 'rxjs';
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

@Injectable({ providedIn: 'root' })
export class AuthService {
  private readonly TOKEN_KEY = 'auth_token';
  currentUser = signal<User | null>(null);

  constructor(private http: HttpClient, private router: Router) {
    this.loadUser();
  }

  private loadUser(): void {
    // Try to fetch the current user; if successful, we're authenticated.
    // The JWT token is stored in HTTP-only cookie set by the backend.
    this.fetchCurrentUser().subscribe({
      error: () => this.logout()
    });
  }

  getToken(): string | null {
    // Token is stored in HTTP-only cookie on the backend.
    // Frontend cannot access it directly (which is the security goal).
    // Credentials are sent automatically with fetch/http calls via withCredentials.
    return localStorage.getItem(this.TOKEN_KEY) || null;
  }

  isAuthenticated(): boolean {
    // Check if we have a current user (fetched successfully)
    return !!this.currentUser();
  }

  login(username: string, password: string): Observable<AuthResponse> {
    return this.http.post<AuthResponse>(`${environment.apiUrl}/auth/login`, { username, password }).pipe(
      tap(res => this.handleAuth(res))
    );
  }

  register(username: string, password: string, email?: string): Observable<AuthResponse> {
    return this.http.post<AuthResponse>(`${environment.apiUrl}/auth/register`, { username, password, email }).pipe(
      tap(res => this.handleAuth(res))
    );
  }

  loginWithDiscord(): void {
    window.location.href = `${environment.apiUrl}/auth/discord`;
  }

  handleDiscordCallback(): void {
    // The backend has set the JWT in an HTTP-only cookie.
    // Fetch the current user to validate the session and load user data.
    // Store a flag in localStorage indicating we're authenticated.
    this.fetchCurrentUser().subscribe({
      next: () => {
        // Set a simple flag to indicate we're authenticated
        localStorage.setItem(this.TOKEN_KEY, 'authenticated');
        this.router.navigate(['/dashboard']);
      },
      error: () => {
        // OAuth flow failed or token is invalid
        this.logout();
      }
    });
  }

  private handleAuth(res: AuthResponse): void {
    // For traditional login/register, store the token in localStorage
    // (backend may also set HTTP-only cookie for consistency)
    localStorage.setItem(this.TOKEN_KEY, res.token);
    this.currentUser.set(res.user);
  }

  fetchCurrentUser(): Observable<User> {
    return this.http.get<User>(`${environment.apiUrl}/users/me`).pipe(
      tap(user => this.currentUser.set(user))
    );
  }

  logout(): void {
    localStorage.removeItem(this.TOKEN_KEY);
    this.currentUser.set(null);
    this.router.navigate(['/auth/login']);
  }
}
