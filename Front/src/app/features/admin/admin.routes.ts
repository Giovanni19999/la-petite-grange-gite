import { Routes } from '@angular/router';

/** Admin area, loaded lazily. Authentication comes with F001. */
export const adminRoutes: Routes = [
  {
    path: '',
    title: 'Administration - La Petite Grange',
    loadComponent: () => import('./dashboard/dashboard.page').then((m) => m.DashboardPage),
  },
];
