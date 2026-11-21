import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

import { IsoDate } from '../../../../shared/dates/dates';

export type BlockedKind = 'closure' | 'family' | 'private_event';

/** A blocked period as the owners describe it (F005). `end` excluded; `null`: until further notice. */
export interface BlockedPeriodInput {
  start: IsoDate;
  end: IsoDate | null;
  kind: BlockedKind;
  label: string | null;
  people: number | null;
  reason: string | null;
}

export interface BlockedPeriod extends BlockedPeriodInput {
  id: number;
  /** `null` when closed until further notice. */
  nights: number | null;
  updated_at: string;
}

export interface PeriodEvent {
  period_id: number;
  action: 'created' | 'updated' | 'deleted';
  before: BlockedPeriodInput | null;
  after: BlockedPeriodInput | null;
  author: string | null;
  at: string;
}

const BASE = '/api/admin/blocked-periods';

/** Blocked periods API (F005). Requires the owner's session. */
@Injectable({ providedIn: 'root' })
export class ClosuresApi {
  private readonly http = inject(HttpClient);

  list(): Observable<BlockedPeriod[]> {
    return this.http.get<BlockedPeriod[]>(BASE);
  }

  create(period: BlockedPeriodInput): Observable<{ id: number }> {
    return this.http.post<{ id: number }>(BASE, period);
  }

  update(id: number, period: BlockedPeriodInput): Observable<void> {
    return this.http.put<void>(`${BASE}/${id}`, period);
  }

  delete(id: number): Observable<void> {
    return this.http.delete<void>(`${BASE}/${id}`);
  }

  history(): Observable<PeriodEvent[]> {
    return this.http.get<PeriodEvent[]>(`${BASE}/history`);
  }
}

/** Names of the kinds in the admin. */
export const KIND_LABELS: Record<BlockedKind, string> = {
  closure: 'Fermeture',
  family: 'Famille',
  private_event: 'Événement privé',
};
