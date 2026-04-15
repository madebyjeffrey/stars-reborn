import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { HttpClient } from '@angular/common/http';
import { environment } from '../../../environments/environment';

interface ApiToken {
  id: string;
  name: string;
  token?: string;
  created_at: string;
  expires_at?: string;
  last_used_at?: string;
}

@Component({
  selector: 'app-api-tokens',
  standalone: true,
  imports: [CommonModule, FormsModule],
  template: `
    <div class="tokens-page">
      <h1>🔑 API Tokens</h1>
      <div class="create-form">
        <h2>Create New Token</h2>
        <div class="form-row">
          <input
            type="text"
            [(ngModel)]="newTokenName"
            placeholder="Token name"
            (keyup.enter)="createToken()"
          />
          <button (click)="createToken()" [disabled]="!newTokenName || creating">
            {{ creating ? 'Creating...' : 'Create Token' }}
          </button>
        </div>
      </div>

      <div *ngIf="newToken" class="new-token-banner">
        <strong>New token created! Save it now — it won't be shown again:</strong>
        <div class="token-value">{{ newToken }}</div>
        <button (click)="copyToken()">Copy</button>
        <button (click)="newToken = null">Dismiss</button>
      </div>

      <div class="tokens-list">
        <h2>Your Tokens</h2>
        <div *ngIf="tokens.length === 0 && !loading" class="empty">No tokens yet.</div>
        <div *ngIf="loading" class="loading">Loading...</div>
        <div *ngFor="let token of tokens" class="token-item">
          <div class="token-info">
            <strong>{{ token.name }}</strong>
            <span class="token-meta">Created: {{ formatDate(token.created_at) }}</span>
            <span *ngIf="token.last_used_at" class="token-meta">Last used: {{ formatDate(token.last_used_at) }}</span>
          </div>
          <button class="delete-btn" (click)="deleteToken(token.id)">Delete</button>
        </div>
      </div>
    </div>
  `,
  styles: [`
    .tokens-page { max-width: 800px; color: white; }
    h1, h2 { color: white; }
    .create-form { background: #1a1a2e; padding: 1.5rem; border-radius: 8px; margin-bottom: 2rem; }
    .form-row { display: flex; gap: 1rem; }
    .form-row input {
      flex: 1;
      padding: 0.75rem;
      border: 1px solid #333;
      border-radius: 4px;
      background: #0f0f1a;
      color: white;
    }
    .form-row button, button {
      padding: 0.75rem 1.5rem;
      background: #e94560;
      color: white;
      border: none;
      border-radius: 4px;
      cursor: pointer;
    }
    .new-token-banner {
      background: #1a3a1a;
      border: 1px solid #4caf50;
      padding: 1rem;
      border-radius: 8px;
      margin-bottom: 2rem;
      color: white;
    }
    .token-value {
      font-family: monospace;
      background: #0f0f1a;
      padding: 0.5rem;
      border-radius: 4px;
      margin: 0.5rem 0;
      word-break: break-all;
    }
    .new-token-banner button { margin-right: 0.5rem; margin-top: 0.5rem; }
    .tokens-list { background: #1a1a2e; padding: 1.5rem; border-radius: 8px; }
    .token-item {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 1rem 0;
      border-bottom: 1px solid #333;
    }
    .token-item:last-child { border-bottom: none; }
    .token-info { display: flex; flex-direction: column; gap: 0.25rem; }
    .token-meta { color: #aaa; font-size: 0.85rem; }
    .delete-btn { background: #c0392b; }
    .empty, .loading { color: #aaa; padding: 1rem 0; }
  `]
})
export class ApiTokensComponent implements OnInit {
  private http = inject(HttpClient);

  tokens: ApiToken[] = [];
  loading = false;
  creating = false;
  newTokenName = '';
  newToken: string | null = null;

  ngOnInit(): void {
    this.loadTokens();
  }

  loadTokens(): void {
    this.loading = true;
    this.http.get<ApiToken[]>(`${environment.apiUrl}/tokens`).subscribe({
      next: (tokens) => {
        this.tokens = tokens;
        this.loading = false;
      },
      error: () => { this.loading = false; }
    });
  }

  createToken(): void {
    if (!this.newTokenName) return;
    this.creating = true;
    this.http.post<ApiToken>(`${environment.apiUrl}/tokens`, { name: this.newTokenName }).subscribe({
      next: (token) => {
        this.newToken = token.token || null;
        this.newTokenName = '';
        this.creating = false;
        this.loadTokens();
      },
      error: () => { this.creating = false; }
    });
  }

  deleteToken(id: string): void {
    if (!confirm('Delete this token?')) return;
    this.http.delete(`${environment.apiUrl}/tokens/${id}`).subscribe({
      next: () => this.loadTokens()
    });
  }

  copyToken(): void {
    if (this.newToken) {
      navigator.clipboard.writeText(this.newToken);
    }
  }

  formatDate(dateStr: string): string {
    return new Date(dateStr).toLocaleDateString();
  }
}
