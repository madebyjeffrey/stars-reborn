import { Component, OnInit, inject } from '@angular/core';
import { AuthService } from '../../../core/auth.service';

@Component({
  selector: 'app-discord-callback',
  standalone: true,
  template: `<div style="text-align:center;padding:2rem;color:white">Processing Discord login...</div>`,
  styles: [`:host { display: block; min-height: 100vh; background: #0f0f1a; }`]
})
export class DiscordCallbackComponent implements OnInit {
  private authService = inject(AuthService);

  ngOnInit(): void {
    // The backend has set the refresh_token in an HTTP-only cookie.
    // Now we need to exchange it for an access token by calling /api/auth/refresh.
    this.authService.handleDiscordCallback();
  }
}
