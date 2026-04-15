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
    const token = this.route.snapshot.queryParamMap.get('token');
    if (token) {
      this.authService.handleDiscordCallback(token);
    }
  }
}
