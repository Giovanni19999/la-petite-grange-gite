import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    pathMatch: 'full',
    title: 'La Petite Grange - Gîte 3 étoiles',
    loadComponent: () => import('./features/public/home/home.page').then((m) => m.HomePage),
  },
  {
    path: 'admin',
    loadChildren: () => import('./features/admin/admin.routes').then((m) => m.adminRoutes),
  },
  {
    path: '**',
    title: 'Page introuvable - La Petite Grange',
    loadComponent: () =>
      import('./features/public/not-found/not-found.page').then((m) => m.NotFoundPage),
  },
];
