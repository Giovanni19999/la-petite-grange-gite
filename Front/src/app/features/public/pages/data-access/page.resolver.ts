import { HttpErrorResponse } from '@angular/common/http';
import { RESPONSE_INIT, inject } from '@angular/core';
import { ResolveFn } from '@angular/router';
import { catchError, of, throwError } from 'rxjs';

import { PagesApi } from './pages.api';
import { Lang, PublicPage } from './pages.models';

/** Language of a public route, set in the route data (ADR-0013). */
export function routeLang(data: Record<string, unknown>): Lang {
  return data['lang'] === 'en' ? 'en' : 'fr';
}

/**
 * Loads the published page of the route before rendering (so the SSR HTML contains it).
 * An unknown page resolves to `null` and sets the HTTP status to 404 on the server.
 */
export const pageResolver: ResolveFn<PublicPage | null> = (route) => {
  const api = inject(PagesApi);
  const response = inject(RESPONSE_INIT, { optional: true });
  const lang = routeLang(route.data);
  const slug = route.paramMap.get('slug');
  const page$ = slug ? api.bySlug(lang, slug) : api.home(lang);

  return page$.pipe(
    catchError((error: unknown) => {
      if (error instanceof HttpErrorResponse && error.status === 404) {
        if (response) {
          response.status = 404;
        }
        return of(null);
      }
      return throwError(() => error);
    }),
  );
};
