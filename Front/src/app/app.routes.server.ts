import { RenderMode, ServerRoute } from '@angular/ssr';

export const serverRoutes: ServerRoute[] = [
  // Admin: private, rendered in the browser only.
  { path: 'admin/**', renderMode: RenderMode.Client },
  // Public pages change when the owners publish: rendered on the server per request
  // (ADR-0013). Unknown pages set the 404 status themselves.
  { path: '**', renderMode: RenderMode.Server },
];
