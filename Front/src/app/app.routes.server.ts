import { RenderMode, ServerRoute } from '@angular/ssr';

export const serverRoutes: ServerRoute[] = [
  // Public pages: pre-rendered for speed and search engines.
  { path: '', renderMode: RenderMode.Prerender },
  // Admin: private, rendered in the browser only.
  { path: 'admin/**', renderMode: RenderMode.Client },
  // Unknown URLs: rendered on the server with a real 404 status.
  { path: '**', renderMode: RenderMode.Server, status: 404 },
];
