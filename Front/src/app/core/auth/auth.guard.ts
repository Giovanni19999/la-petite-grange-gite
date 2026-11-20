import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';

import { AuthStore } from './auth.store';

/** Path of the admin login page. */
export const LOGIN_PATH = '/admin/connexion';

/** Lets logged-in owners through; sends others to the login page, remembering where they were going. */
export const authGuard: CanActivateFn = async (_route, state) => {
  const store = inject(AuthStore);
  const router = inject(Router);

  await store.ensureSessionChecked();
  if (store.isLoggedIn()) {
    return true;
  }
  return router.createUrlTree([LOGIN_PATH], { queryParams: { retour: state.url } });
};

/** Only internal admin URLs are accepted as a return address (no open redirect). */
export function safeReturnUrl(candidate: string | null | undefined): string {
  if (candidate?.startsWith('/admin') && !candidate.startsWith(LOGIN_PATH)) {
    return candidate;
  }
  return '/admin';
}
