import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

import { IsoDate } from '../../../../shared/dates/dates';

/** Price of a channel per night, in cents: what the guest pays and what the owners keep. */
export interface ChannelPrice {
  listed: number;
  net: number;
}

export interface ChannelRates {
  commission_bp: number;
  payment_fees_bp: number;
  vat_on_commission: boolean;
}

export interface Channel extends ChannelRates {
  code: string;
  name: string;
  /** Share of the listed price the channel keeps, in basis points. */
  effective_bp: number;
}

export interface PricingSettings {
  cleaning_fee: number;
  default_min_nights: number;
  commission_alert_points: number;
}

/** Totals of a typical stay per channel, `[listed, net]` in cents, `null` when incomplete. */
export type TypicalStays = Record<string, Record<string, [number, number] | null>>;

export interface PricedRange {
  id: number;
  start: IsoDate;
  end: IsoDate;
  label: string | null;
  min_nights: number;
  prices: Record<string, ChannelPrice>;
  typical_stays: TypicalStays;
}

export interface PriceBook {
  channels: Channel[];
  settings: PricingSettings;
  defaults: Record<string, ChannelPrice>;
  default_typical_stays: TypicalStays;
  ranges: PricedRange[];
  /** Channels without default price. */
  incomplete: string[];
}

export interface RangeRequest {
  start: IsoDate;
  end: IsoDate;
  label: string | null;
  min_nights: number;
  prices: Record<string, ChannelPrice>;
}

export interface NightPrices {
  date: IsoDate;
  min_nights: number;
  prices: Record<string, ChannelPrice | null>;
}

export interface PriceChangeEntry {
  description: string;
  author: string | null;
  at: string;
}

const BASE = '/api/admin/prices';

/** Prices API (F006). Requires the owner's session. */
@Injectable({ providedIn: 'root' })
export class PricesApi {
  private readonly http = inject(HttpClient);

  book(): Observable<PriceBook> {
    return this.http.get<PriceBook>(BASE);
  }

  saveSettings(settings: PricingSettings): Observable<void> {
    return this.http.put<void>(`${BASE}/settings`, settings);
  }

  saveRates(code: string, rates: ChannelRates): Observable<void> {
    return this.http.put<void>(`${BASE}/channels/${code}`, rates);
  }

  /** Every channel of the default table at once (`null`: no default price). */
  saveDefaults(prices: Record<string, ChannelPrice | null>): Observable<void> {
    return this.http.put<void>(`${BASE}/defaults`, prices);
  }

  createRange(range: RangeRequest): Observable<void> {
    return this.http.post<void>(`${BASE}/ranges`, range);
  }

  updateRange(id: number, range: RangeRequest): Observable<void> {
    return this.http.put<void>(`${BASE}/ranges/${id}`, range);
  }

  deleteRange(id: number): Observable<void> {
    return this.http.delete<void>(`${BASE}/ranges/${id}`);
  }

  copyYear(year: number): Observable<{ copied: number }> {
    return this.http.post<{ copied: number }>(`${BASE}/copy-year`, { year });
  }

  copyChannel(from: string, to: string): Observable<void> {
    return this.http.post<void>(`${BASE}/copy-channel`, { from, to });
  }

  nights(from: IsoDate, to: IsoDate): Observable<NightPrices[]> {
    return this.http.get<NightPrices[]>(`${BASE}/nights`, { params: { from, to } });
  }

  history(): Observable<PriceChangeEntry[]> {
    return this.http.get<PriceChangeEntry[]>(`${BASE}/history`);
  }
}
