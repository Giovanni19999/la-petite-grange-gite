import { HttpClient, HttpParams } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

import { IsoDate } from '../../../../shared/dates/dates';

export type BookingStatus = 'confirmee' | 'acompte_recu' | 'soldee' | 'annulee';

export const STATUS_LABELS: Record<BookingStatus, string> = {
  confirmee: 'Confirmée, acompte attendu',
  acompte_recu: 'Acompte reçu',
  soldee: 'Soldée',
  annulee: 'Annulée',
};

/** A line of the owners' list (amounts in cents). */
export interface BookingSummary {
  id: number;
  reference: string;
  status: BookingStatus;
  arrival: IsoDate;
  departure: IsoDate;
  guests: number;
  guest: string;
  stay_total: number;
  deposit: number;
  paid: number;
  /** Deposit not received after the alert delay. */
  deposit_late: boolean;
  /** Balance due date passed and not paid. */
  balance_late: boolean;
  created_at: string;
}

export interface Guest {
  first_name: string;
  last_name: string;
  email: string;
  phone: string;
  street: string;
  postcode: string;
  town: string;
  country: string;
}

/** Amounts copied on the booking when it was made. */
export interface BookingAmounts {
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
  /** `null` when everything was due at booking. */
  balance_due: IsoDate | null;
  tourist_tax: number;
  paid: number;
}

export interface BookingDetail {
  id: number;
  reference: string;
  status: BookingStatus;
  payment_mode: 'offline' | 'online';
  lang: 'fr' | 'en';
  arrival: IsoDate;
  departure: IsoDate;
  adults: number;
  children: number;
  guest: Guest;
  arrival_time: number;
  arrival_slots: number[];
  message: string;
  amounts: BookingAmounts;
  deposit_percent: number | null;
  terms: { page: number; version: number; accepted_at: string };
  tracking_link: string;
  created_at: string;
  cancelled_at: string | null;
  cancel_reason: string | null;
  deposit_alert_sent_at: string | null;
  payments: {
    amount: number;
    paid_on: IsoDate;
    method: string;
    note: string;
    author: string | null;
    created_at: string;
  }[];
  events: { description: string; author: string | null; at: string }[];
  messages: {
    id: number;
    channel: 'email' | 'sms';
    recipient: string;
    summary: string;
    status: string;
    error: string | null;
    created_at: string;
    sent_at: string | null;
  }[];
}

export interface PaymentInput {
  amount: number;
  paid_on: IsoDate;
  method: string;
  note: string;
}

export interface BookingFilter {
  status?: BookingStatus | '';
  from?: IsoDate | '';
  to?: IsoDate | '';
}

const BASE = '/api/admin/bookings';

/** Owners' bookings API (F007). Requires the owner's session. */
@Injectable({ providedIn: 'root' })
export class BookingsApi {
  private readonly http = inject(HttpClient);

  list(filter: BookingFilter = {}): Observable<BookingSummary[]> {
    let params = new HttpParams();
    for (const [key, value] of Object.entries(filter)) {
      if (value) {
        params = params.set(key, value as string);
      }
    }
    return this.http.get<BookingSummary[]>(BASE, { params });
  }

  detail(id: number): Observable<BookingDetail> {
    return this.http.get<BookingDetail>(`${BASE}/${id}`);
  }

  recordPayment(id: number, payment: PaymentInput): Observable<{ status: BookingStatus }> {
    return this.http.post<{ status: BookingStatus }>(`${BASE}/${id}/payments`, payment);
  }

  cancel(id: number, reason: string): Observable<void> {
    return this.http.post<void>(`${BASE}/${id}/cancel`, { reason });
  }

  updateGuest(id: number, guest: Guest, arrivalTime: number): Observable<void> {
    return this.http.put<void>(`${BASE}/${id}/guest`, { guest, arrival_time: arrivalTime });
  }
}
