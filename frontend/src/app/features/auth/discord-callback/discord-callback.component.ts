import { Component, OnInit, inject } from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { AuthService } from '../../../core/auth.service';

@Component({
  selector: 'app-discord-callback',
  standalone: true,
  template: `<div style="text-align:center;padding:2rem;color:white">Processing Discord login...</div>`,
  styles: [`:host { display: block; min-height: 100vh; background: #0f0f1a; }`]
})
export class DiscordCallbackComponent implements OnInit {
  private route = inject(ActivatedRoute);
  private authService = inject(AuthService);

  ngOnInit(): void {
    // The JWT token is transmitted via HTTP-only cookie from the backend,
    // not in the URL query parameters (which would leak via history, referrers, logs).
    // Just fetch the current user to verify authentication and establish the session.
    this.authService.handleDiscordCallback();
  }
}
