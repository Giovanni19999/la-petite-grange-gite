import { inject } from '@angular/core';
import { ResolveFn } from '@angular/router';
import { catchError, of } from 'rxjs';

import { routeLang } from './page.resolver';
import { PagesApi } from './pages.api';
import { MenuItem } from './pages.models';

/** Menu of the route language; an empty menu if the API is unavailable. */
export const menuResolver: ResolveFn<MenuItem[]> = (route) =>
  inject(PagesApi)
    .menu(routeLang(route.data))
    .pipe(catchError(() => of([])));
