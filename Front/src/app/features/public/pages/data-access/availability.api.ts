import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

import { IsoDate } from '../../../../shared/dates/dates';

/** A night of the public calendar (F004): `price` (site, in cents) is `null` when unavailable. */
export interface CalendarNight {
  date: IsoDate;
  price: number | null;
  /** Minimum stay when arriving this night (free nights only). */
  min_nights?: number;
}

export interface PublicCalendar {
  earliest_arrival: IsoDate;
  last_night: IsoDate;
  capacity: number;
  /** Guests under this age count as children. */
  children_under: number;
  /** ISO weekday numbers, Monday = 1. */
  arrival_weekdays: number[];
  departure_weekdays: number[];
  nights: CalendarNight[];
}

export interface StayRequest {
  arrival: IsoDate;
  departure: IsoDate;
  adults: number;
  children: number;
}

/** Exact quote, computed by the API only (amounts in cents). */
export interface Quote {
  nights: number;
  nights_total: number;
  cleaning_fee: number;
  stay_total: number;
  deposit: number;
  balance: number;
  /** `null` when everything is due at booking. */
  balance_due: IsoDate | null;
  /** Paid on arrival, apart from the stay total. */
  tourist_tax: number;
}

/** Public calendar and quote (F004). */
@Injectable({ providedIn: 'root' })
export class AvailabilityApi {
  private readonly http = inject(HttpClient);

  calendar(from: IsoDate, to: IsoDate): Observable<PublicCalendar> {
    return this.http.get<PublicCalendar>('/api/calendar', { params: { from, to } });
  }

  quote(stay: StayRequest): Observable<Quote> {
    return this.http.post<Quote>('/api/quote', stay);
  }
}
