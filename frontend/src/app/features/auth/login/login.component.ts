import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Router, RouterLink } from '@angular/router';
import { AuthService } from '../../../core/auth.service';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterLink],
  template: `
    <div class="auth-container">
      <div class="auth-card">
        <h1>⭐ Stars Reborn</h1>
        <h2>Login</h2>
        <div *ngIf="error" class="error">{{ error }}</div>
        <form (ngSubmit)="onSubmit()">
          <div class="form-group">
            <label>Username</label>
            <input type="text" [(ngModel)]="username" name="username" required autocomplete="username" />
          </div>
          <div class="form-group">
            <label>Password</label>
            <input type="password" [(ngModel)]="password" name="password" required autocomplete="current-password" />
          </div>
          <button type="submit" [disabled]="loading">
            {{ loading ? 'Logging in...' : 'Login' }}
          </button>
        </form>
        <div class="divider">or</div>
        <button class="discord-btn" (click)="loginWithDiscord()">
          Login with Discord
        </button>
        <p>Don't have an account? <a routerLink="/auth/register">Register</a></p>
      </div>
    </div>
  `,
  styles: [`
    .auth-container {
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      background: #0f0f1a;
    }
    .auth-card {
      background: #1a1a2e;
      padding: 2rem;
      border-radius: 8px;
      width: 100%;
      max-width: 400px;
      color: white;
    }
    h1 { text-align: center; margin-bottom: 0.5rem; }
    h2 { text-align: center; margin-bottom: 1.5rem; color: #ccc; }
    .form-group { margin-bottom: 1rem; }
    .form-group label { display: block; margin-bottom: 0.25rem; color: #aaa; }
    .form-group input {
      width: 100%;
      padding: 0.75rem;
      border: 1px solid #333;
      border-radius: 4px;
      background: #0f0f1a;
      color: white;
      box-sizing: border-box;
    }
    button[type=submit] {
      width: 100%;
      padding: 0.75rem;
      background: #e94560;
      color: white;
      border: none;
      border-radius: 4px;
      cursor: pointer;
      font-size: 1rem;
      margin-top: 0.5rem;
    }
    .divider { text-align: center; margin: 1rem 0; color: #666; }
    .discord-btn {
      width: 100%;
      padding: 0.75rem;
      background: #5865f2;
      color: white;
      border: none;
      border-radius: 4px;
      cursor: pointer;
      font-size: 1rem;
    }
    .error { background: #e9456022; color: #e94560; padding: 0.75rem; border-radius: 4px; margin-bottom: 1rem; }
    p { text-align: center; margin-top: 1rem; color: #aaa; }
    a { color: #e94560; }
  `]
})
export class LoginComponent {
  username = '';
  password = '';
  loading = false;
  error = '';

  private authService = inject(AuthService);
  private router = inject(Router);

  onSubmit(): void {
    if (!this.username || !this.password) return;
    this.loading = true;
    this.error = '';

    this.authService.login(this.username, this.password).subscribe({
      next: () => this.router.navigate(['/dashboard']),
      error: (err) => {
        this.error = this.getErrorMessage(err);
        this.loading = false;
      }
    });
  }

  loginWithDiscord(): void {
    this.authService.loginWithDiscord();
  }

  private getErrorMessage(err: any): string {
    // Check for new error response format with code field
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
    // Fallback to error message if no code field
    return err.error?.error || 'Login failed';
  }
}
