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

@Injectable({ providedIn: 'root' })
export class AuthService {
  private readonly TOKEN_KEY = 'auth_token';
  currentUser = signal<User | null>(null);

  constructor(private http: HttpClient, private router: Router) {
    this.loadUser();
  }

  private loadUser(): void {
    const token = this.getToken();
    if (token) {
      this.fetchCurrentUser().subscribe({
        error: () => this.logout()
      });
    }
  }

  getToken(): string | null {
    return localStorage.getItem(this.TOKEN_KEY);
  }

  isAuthenticated(): boolean {
    return !!this.getToken();
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

  handleDiscordCallback(token: string): void {
    localStorage.setItem(this.TOKEN_KEY, token);
    this.fetchCurrentUser().subscribe(() => {
      this.router.navigate(['/dashboard']);
    });
  }

  private handleAuth(res: AuthResponse): void {
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
