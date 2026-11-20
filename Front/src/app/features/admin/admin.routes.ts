import { Routes } from '@angular/router';

import { authGuard } from '../../core/auth/auth.guard';

/** Admin area, loaded lazily. Every page except the login page requires a session (F001). */
export const adminRoutes: Routes = [
  {
    path: 'connexion',
    title: 'Connexion - La Petite Grange',
    loadComponent: () => import('./login/login.page').then((m) => m.LoginPage),
  },
  {
    path: '',
    canActivate: [authGuard],
    title: 'Administration - La Petite Grange',
    loadComponent: () => import('./dashboard/dashboard.page').then((m) => m.DashboardPage),
  },
];
