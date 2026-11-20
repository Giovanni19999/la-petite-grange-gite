import { Routes } from '@angular/router';

import { menuResolver } from './features/public/pages/data-access/menu.resolver';
import { pageResolver } from './features/public/pages/data-access/page.resolver';

/** Home page and pages by slug, in one language (ADR-0013). */
function publicPages(lang: 'fr' | 'en'): Routes {
  const page = () => import('./features/public/pages/page.page').then((m) => m.PageView);
  return [
    {
      path: '',
      loadComponent: () =>
        import('./features/public/pages/public-layout').then((m) => m.PublicLayout),
      data: { lang },
      resolve: { menu: menuResolver },
      children: [
        { path: '', pathMatch: 'full', resolve: { page: pageResolver }, loadComponent: page },
        { path: ':slug', resolve: { page: pageResolver }, loadComponent: page },
      ],
    },
  ];
}

export const routes: Routes = [
  {
    path: 'admin',
    loadChildren: () => import('./features/admin/admin.routes').then((m) => m.adminRoutes),
  },
  { path: 'en', children: publicPages('en') },
  ...publicPages('fr'),
  {
    path: '**',
    title: 'Page introuvable - La Petite Grange',
    loadComponent: () =>
      import('./features/public/not-found/not-found.page').then((m) => m.NotFoundPage),
  },
];
