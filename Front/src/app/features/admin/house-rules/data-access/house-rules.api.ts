import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

import { IsoDate } from '../../../../shared/dates/dates';

/** Deposit asked at booking: a percentage of the stay or a fixed amount in cents. */
export type Deposit = { mode: 'percentage'; percent: number } | { mode: 'fixed'; amount: number };

/** House rules of F004 and F007, as the API stores them (amounts in cents, rates in basis
 * points, times of day in minutes after midnight). */
export interface HouseRules {
  capacity: number;
  min_delay_days: number;
  /** ISO weekday numbers, Monday = 1. */
  arrival_weekdays: number[];
  departure_weekdays: number[];
  deposit: Deposit;
  balance_days: number;
  tourist_tax: { per_adult_night: number; additional_bp: number; exempt_under: number };
  horizon: { months: number; until: IsoDate | null };
  arrival_from: number;
  departure_before: number;
  pets_allowed: boolean;
  /** Days without deposit before the owners are alerted. */
  deposit_alert_days: number;
  /** Page of the rental terms; booking is closed without it. */
  terms_page: number | null;
}

export interface RuleChangeEntry {
  description: string;
  author: string | null;
  at: string;
}

const BASE = '/api/admin/house-rules';

/** House rules API (F004). Requires the owner's session. */
@Injectable({ providedIn: 'root' })
export class HouseRulesApi {
  private readonly http = inject(HttpClient);

  rules(): Observable<HouseRules> {
    return this.http.get<HouseRules>(BASE);
  }

  save(rules: HouseRules): Observable<void> {
    return this.http.put<void>(BASE, rules);
  }

  history(): Observable<RuleChangeEntry[]> {
    return this.http.get<RuleChangeEntry[]>(`${BASE}/history`);
  }
}
