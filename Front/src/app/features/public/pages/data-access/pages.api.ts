import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

import { Lang, MenuItem, PublicPage } from './pages.models';

/** Public pages API (F002): published content only. */
@Injectable({ providedIn: 'root' })
export class PagesApi {
  private readonly http = inject(HttpClient);

  menu(lang: Lang): Observable<MenuItem[]> {
    return this.http.get<MenuItem[]>('/api/pages/menu', { params: { lang } });
  }

  home(lang: Lang): Observable<PublicPage> {
    return this.http.get<PublicPage>('/api/pages/home', { params: { lang } });
  }

  bySlug(lang: Lang, slug: string): Observable<PublicPage> {
    return this.http.get<PublicPage>(`/api/pages/by-slug/${encodeURIComponent(slug)}`, {
      params: { lang },
    });
  }
}
