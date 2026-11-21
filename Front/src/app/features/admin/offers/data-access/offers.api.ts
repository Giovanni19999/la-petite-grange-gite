import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

import { IsoDate } from '../../../../shared/dates/dates';

export interface Names {
  fr: string;
  en: string;
}

/** Discount of a promotion: basis points of the nights, or cents per stay / per night. */
export type Discount =
  | { kind: 'percent'; bp: number }
  | { kind: 'per_stay'; amount: number }
  | { kind: 'per_night'; amount: number };

export interface Promotion {
  name: Names;
  discount: Discount;
  min_nights: number | null;
  max_nights: number | null;
  /** On the arrival date, both included. */
  valid_from: IsoDate | null;
  valid_until: IsoDate | null;
  active: boolean;
}

export type OptionUnit = 'stay' | 'night' | 'guest' | 'guest_night';

export interface StayOption {
  name: Names;
  description: Names;
  unit: OptionUnit;
  /** In cents; 0 for an offered option. */
  price: number;
  min_nights: number | null;
  max_nights: number | null;
  position: number;
  active: boolean;
}

export type StoredPromotion = Promotion & { id: number };
export type StoredOption = StayOption & { id: number };

export interface Offers {
  promotions: StoredPromotion[];
  options: StoredOption[];
}

const BASE = '/api/admin/offers';

/** Promotions and options API (F026). Requires the owner's session. */
@Injectable({ providedIn: 'root' })
export class OffersApi {
  private readonly http = inject(HttpClient);

  list(): Observable<Offers> {
    return this.http.get<Offers>(BASE);
  }

  createPromotion(promotion: Promotion): Observable<{ id: number }> {
    return this.http.post<{ id: number }>(`${BASE}/promotions`, promotion);
  }

  updatePromotion(id: number, promotion: Promotion): Observable<void> {
    return this.http.put<void>(`${BASE}/promotions/${id}`, promotion);
  }

  deletePromotion(id: number): Observable<void> {
    return this.http.delete<void>(`${BASE}/promotions/${id}`);
  }

  createOption(option: StayOption): Observable<{ id: number }> {
    return this.http.post<{ id: number }>(`${BASE}/options`, option);
  }

  updateOption(id: number, option: StayOption): Observable<void> {
    return this.http.put<void>(`${BASE}/options/${id}`, option);
  }

  deleteOption(id: number): Observable<void> {
    return this.http.delete<void>(`${BASE}/options/${id}`);
  }
}
