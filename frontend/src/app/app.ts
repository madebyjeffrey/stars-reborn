import { Component, inject } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive } from '@angular/router';
import { CommonModule } from '@angular/common';
import { AuthService } from './core/auth.service';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [RouterOutlet, RouterLink, RouterLinkActive, CommonModule],
  template: `
    <nav class="navbar" *ngIf="authService.isAuthenticated()">
      <div class="navbar-brand">
        <a routerLink="/dashboard">⭐ Stars Reborn</a>
      </div>
      <div class="navbar-menu">
        <a routerLink="/dashboard" routerLinkActive="active">Dashboard</a>
        <a routerLink="/tokens" routerLinkActive="active">API Tokens</a>
        <button (click)="authService.logout()">Logout</button>
      </div>
    </nav>
    <main class="main-content">
      <router-outlet></router-outlet>
    </main>
  `,
  styles: [`
    .navbar {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 1rem 2rem;
      background: #1a1a2e;
      color: white;
    }
    .navbar-brand a {
      color: white;
      text-decoration: none;
      font-size: 1.5rem;
      font-weight: bold;
    }
    .navbar-menu { display: flex; gap: 1rem; align-items: center; }
    .navbar-menu a { color: #ccc; text-decoration: none; }
    .navbar-menu a.active { color: white; font-weight: bold; }
    .navbar-menu button {
      background: #e94560;
      color: white;
      border: none;
      padding: 0.5rem 1rem;
      border-radius: 4px;
      cursor: pointer;
    }
    .main-content { padding: 2rem; max-width: 1200px; margin: 0 auto; }
  `]
})
export class App {
  authService = inject(AuthService);
}
