import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { AuthService } from '../../core/auth.service';

@Component({
  selector: 'app-dashboard',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <div class="dashboard">
      <h1>Dashboard</h1>
      <div class="user-card" *ngIf="user()">
        <div class="avatar" *ngIf="user()?.discord_avatar">
          <img [src]="getAvatarUrl()" [alt]="user()?.username" />
        </div>
        <div class="user-info">
          <h2>{{ user()?.username }}</h2>
          <p *ngIf="user()?.email">📧 {{ user()?.email }}</p>
          <p *ngIf="user()?.discord_username">🎮 Discord: {{ user()?.discord_username }}</p>
        </div>
      </div>
      <div class="quick-links">
        <a routerLink="/tokens" class="link-card">
          <h3>🔑 API Tokens</h3>
          <p>Manage your API access tokens</p>
        </a>
      </div>
    </div>
  `,
  styles: [`
    .dashboard { max-width: 800px; }
    h1 { color: white; margin-bottom: 2rem; }
    .user-card {
      background: #1a1a2e;
      padding: 1.5rem;
      border-radius: 8px;
      display: flex;
      align-items: center;
      gap: 1.5rem;
      margin-bottom: 2rem;
      color: white;
    }
    .avatar img { width: 80px; height: 80px; border-radius: 50%; }
    .user-info h2 { margin: 0 0 0.5rem; }
    .user-info p { margin: 0.25rem 0; color: #aaa; }
    .quick-links { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 1rem; }
    .link-card {
      background: #1a1a2e;
      padding: 1.5rem;
      border-radius: 8px;
      text-decoration: none;
      color: white;
      transition: background 0.2s;
    }
    .link-card:hover { background: #2a2a4e; }
    .link-card h3 { margin: 0 0 0.5rem; }
    .link-card p { margin: 0; color: #aaa; font-size: 0.9rem; }
  `]
})
export class DashboardComponent {
  authService = inject(AuthService);
  user = this.authService.currentUser;

  getAvatarUrl(): string {
    const u = this.user();
    if (u?.discord_id && u?.discord_avatar) {
      return `https://cdn.discordapp.com/avatars/${u.discord_id}/${u.discord_avatar}.png`;
    }
    return '';
  }
}
