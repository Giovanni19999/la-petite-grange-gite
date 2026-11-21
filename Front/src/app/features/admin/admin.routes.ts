import { CanDeactivateFn, Routes } from '@angular/router';

import { authGuard } from '../../core/auth/auth.guard';
import type { PageEditorPage } from './pages/editor/page-editor.page';

/** Leaving the page editor saves what is pending and asks before losing changes (F023). */
const leaveEditor: CanDeactivateFn<PageEditorPage> = (editor) => editor.canLeave();

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
    children: [
      {
        path: '',
        pathMatch: 'full',
        title: 'Administration - La Petite Grange',
        loadComponent: () => import('./dashboard/dashboard.page').then((m) => m.DashboardPage),
      },
      {
        path: 'pages',
        title: 'Pages du site - La Petite Grange',
        loadComponent: () =>
          import('./pages/pages-list/pages-list.page').then((m) => m.PagesListPage),
      },
      {
        path: 'fermetures',
        title: 'Fermetures - La Petite Grange',
        loadComponent: () => import('./closures/closures.page').then((m) => m.ClosuresPage),
      },
      {
        path: 'tarifs',
        title: 'Tarifs - La Petite Grange',
        loadComponent: () => import('./prices/prices.page').then((m) => m.PricesPage),
      },
      {
        path: 'medias',
        title: 'Médiathèque - La Petite Grange',
        loadComponent: () => import('./media/media-library.page').then((m) => m.MediaLibraryPage),
      },
      {
        path: 'pages/:id',
        title: 'Modifier la page - La Petite Grange',
        canDeactivate: [leaveEditor],
        loadComponent: () =>
          import('./pages/editor/page-editor.page').then((m) => m.PageEditorPage),
      },
    ],
  },
];
