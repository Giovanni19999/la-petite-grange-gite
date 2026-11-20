import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

import {
  AdminPage,
  AdminPageSummary,
  PageChanges,
  PageContent,
  PageVersion,
} from './admin-pages.models';

const BASE = '/api/admin/pages';

/** Admin pages API of F002, used by the editor (F023). Requires the owner's session. */
@Injectable({ providedIn: 'root' })
export class AdminPagesApi {
  private readonly http = inject(HttpClient);

  list(): Observable<AdminPageSummary[]> {
    return this.http.get<AdminPageSummary[]>(BASE);
  }

  create(content: PageContent): Observable<{ id: number }> {
    return this.http.post<{ id: number }>(BASE, { content });
  }

  get(id: number): Observable<AdminPage> {
    return this.http.get<AdminPage>(`${BASE}/${id}`);
  }

  delete(id: number): Observable<void> {
    return this.http.delete<void>(`${BASE}/${id}`);
  }

  /** Saves the draft based on `draftVersion`; returns the new draft version (409 if stale). */
  saveDraft(
    id: number,
    draftVersion: number,
    content: PageContent,
  ): Observable<{ draft_version: number }> {
    return this.http.put<{ draft_version: number }>(`${BASE}/${id}/draft`, {
      draft_version: draftVersion,
      content,
    });
  }

  changes(id: number): Observable<PageChanges> {
    return this.http.get<PageChanges>(`${BASE}/${id}/changes`);
  }

  /** Publishes the draft version the owner confirmed (409 if it changed meanwhile). */
  publish(id: number, draftVersion: number): Observable<void> {
    return this.http.post<void>(`${BASE}/${id}/publish`, { draft_version: draftVersion });
  }

  versions(id: number): Observable<PageVersion[]> {
    return this.http.get<PageVersion[]>(`${BASE}/${id}/versions`);
  }

  restore(id: number, version: number): Observable<{ draft_version: number }> {
    return this.http.post<{ draft_version: number }>(
      `${BASE}/${id}/versions/${version}/restore`,
      null,
    );
  }
}
