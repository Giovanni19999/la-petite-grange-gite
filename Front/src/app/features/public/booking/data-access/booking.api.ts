import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

import { IsoDate } from '../../../../shared/dates/dates';
import { Lang } from '../../pages/data-access/pages.models';

/** What the booking page shows besides the quote (F007). */
export interface BookingInfo {
  open: boolean;
  capacity: number;
  pets_allowed: boolean;
  /** Minutes after midnight. */
  arrival_from: number;
  departure_before: number;
  arrival_slots: number[];
  /** Payment instructions, sanitised HTML. */
  instructions: string;
  /** A bank details document is attached to the confirmation email. */
  document: boolean;
  terms: { slug: string; title: string } | null;
}

export interface GuestDetails {
  first_name: string;
  last_name: string;
  email: string;
  phone: string;
  street: string;
  postcode: string;
  town: string;
  country: string;
}

export interface BookingForm {
  arrival: IsoDate;
  departure: IsoDate;
  adults: number;
  children: number;
  options: number[];
  lang: Lang;
  guest: GuestDetails;
  arrival_time: number;
  message: string;
  terms_accepted: boolean;
  /** Hidden field, left empty by people. */
  website: string;
}

/** The guest's view of their booking (tracking page). */
export interface TrackedBooking {
  reference: string;
  status: 'confirmee' | 'acompte_recu' | 'soldee' | 'annulee';
  lang: Lang;
  arrival: IsoDate;
  departure: IsoDate;
  adults: number;
  children: number;
  first_name: string;
  arrival_time: number;
  arrival_slots: number[];
  arrival_time_changeable: boolean;
  arrival_from: number;
  departure_before: number;
  pets_allowed: boolean;
  amounts: {
    nights: number;
    nights_total: number;
    promotion: string | null;
    discount: number;
    cleaning_fee: number;
    options: { name: string; price: number }[];
    options_total: number;
    stay_total: number;
    deposit: number;
    balance: number;
    balance_due: IsoDate | null;
    tourist_tax: number;
    paid: number;
  };
  instructions: string;
}

/** Public booking API (F007). */
@Injectable({ providedIn: 'root' })
export class BookingApi {
  private readonly http = inject(HttpClient);

  info(lang: Lang): Observable<BookingInfo> {
    return this.http.get<BookingInfo>('/api/booking-info', { params: { lang } });
  }

  book(form: BookingForm): Observable<{ reference: string; token: string } | null> {
    return this.http.post<{ reference: string; token: string } | null>('/api/bookings', form);
  }

  track(token: string): Observable<TrackedBooking> {
    return this.http.get<TrackedBooking>(`/api/bookings/${encodeURIComponent(token)}`);
  }

  changeArrivalTime(token: string, minutes: number): Observable<void> {
    return this.http.put<void>(`/api/bookings/${encodeURIComponent(token)}/arrival-time`, {
      arrival_time: minutes,
    });
  }
}
