import { Routes } from '@angular/router';
import { authGuard } from './core/auth.guard';

export const routes: Routes = [
  { path: '', redirectTo: 'dashboard', pathMatch: 'full' },
  {
    path: 'auth',
    children: [
      {
        path: 'login',
        loadComponent: () => import('./features/auth/login/login.component').then(m => m.LoginComponent)
      },
      {
        path: 'register',
        loadComponent: () => import('./features/auth/register/register.component').then(m => m.RegisterComponent)
      },
      {
        path: 'discord/callback',
        loadComponent: () => import('./features/auth/discord-callback/discord-callback.component').then(m => m.DiscordCallbackComponent)
      }
    ]
  },
  {
    path: 'dashboard',
    canActivate: [authGuard],
    loadComponent: () => import('./features/dashboard/dashboard.component').then(m => m.DashboardComponent)
  },
  {
    path: 'tokens',
    canActivate: [authGuard],
    loadComponent: () => import('./features/api-tokens/api-tokens.component').then(m => m.ApiTokensComponent)
  },
  { path: '**', redirectTo: 'dashboard' }
];
