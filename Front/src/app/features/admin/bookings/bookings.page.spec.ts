import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter, withComponentInputBinding } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';
import { vi } from 'vitest';

import { BookingDetailPage } from './booking-detail.page';
import { BookingsListPage, paymentState } from './bookings-list.page';
import { BookingDetail, BookingSummary } from './data-access/bookings.api';

const SUMMARY: BookingSummary = {
  id: 1,
  reference: 'LPG-2027-0001',
  status: 'confirmee',
  arrival: '2027-07-10',
  departure: '2027-07-14',
  guests: 2,
  guest: 'Léa Martin',
  stay_total: 52_000,
  deposit: 26_000,
  paid: 0,
  deposit_late: true,
  balance_late: false,
  created_at: '2027-03-01T10:00:00Z',
};

const DETAIL: BookingDetail = {
  id: 1,
  reference: 'LPG-2027-0001',
  status: 'confirmee',
  payment_mode: 'offline',
  lang: 'fr',
  arrival: '2027-07-10',
  departure: '2027-07-14',
  adults: 2,
  children: 0,
  guest: {
    first_name: 'Léa',
    last_name: 'Martin',
    email: 'lea@example.test',
    phone: '+33612345678',
    street: '1 rue des Lilas',
    postcode: '75001',
    town: 'Paris',
    country: 'France',
  },
  arrival_time: 1080,
  arrival_slots: [960, 990, 1020, 1050, 1080],
  message: '',
  amounts: {
    nights: 4,
    nights_total: 40_000,
    promotion: 'Bienvenue',
    discount: 4_000,
    cleaning_fee: 12_000,
    options: [],
    options_total: 0,
    stay_total: 48_000,
    deposit: 24_000,
    balance: 24_000,
    balance_due: '2027-06-10',
    tourist_tax: 1_200,
    paid: 0,
  },
  deposit_percent: 50,
  terms: { page: 2, version: 1, accepted_at: '2027-03-01T10:00:00Z' },
  tracking_link: 'https://gite.example.test/reservation/abc',
  created_at: '2027-03-01T10:00:00Z',
  cancelled_at: null,
  cancel_reason: null,
  deposit_alert_sent_at: null,
  payments: [],
  events: [{ description: 'Réservation faite en ligne', author: null, at: '2027-03-01T10:00:00Z' }],
  messages: [],
};

describe('F007 payment state of the list', () => {
  it('says what is expected', () => {
    expect(paymentState(SUMMARY)).toBe('Acompte en retard');
    expect(paymentState({ ...SUMMARY, deposit_late: false })).toBe('Acompte attendu');
    expect(paymentState({ ...SUMMARY, paid: 26_000, balance_late: true })).toBe('Solde en retard');
    expect(paymentState({ ...SUMMARY, paid: 52_000 })).toBe('Tout est payé');
    expect(paymentState({ ...SUMMARY, status: 'annulee' })).toBe('—');
  });
});

describe('F007 bookings screens', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter(
          [
            { path: 'admin/reservations', component: BookingsListPage },
            { path: 'admin/reservations/:id', component: BookingDetailPage },
          ],
          withComponentInputBinding(),
        ),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  const page = () => harness.routeNativeElement as HTMLElement;
  const button = (text: string) =>
    Array.from(page().querySelectorAll<HTMLButtonElement>('button')).find(
      (candidate) => candidate.textContent?.trim() === text,
    )!;

  async function openDetail(detail = DETAIL): Promise<void> {
    await harness.navigateByUrl('/admin/reservations/1');
    await settle();
    http.expectOne('/api/admin/payment-info').flush({
      instructions: { fr: '', en: '' },
      methods: ['Virement', 'Chèque'],
      document: null,
      provider: 'none',
    });
    http.expectOne('/api/admin/bookings/1').flush(detail);
    await settle();
  }

  it('lists the bookings with their payment state', async () => {
    await harness.navigateByUrl('/admin/reservations');
    await settle();
    http.expectOne('/api/admin/bookings').flush([SUMMARY]);
    await settle();
    const row = page().querySelector('[data-booking="1"]')!.textContent!;
    expect(row).toContain('LPG-2027-0001');
    expect(row).toContain('Acompte en retard');
  });

  it('F007-S08 F026-S05 records a payment and shows the frozen conditions', async () => {
    await openDetail();
    expect(page().textContent).toContain('Promotion « Bienvenue »');
    expect(page().querySelector<HTMLInputElement>('[name="amount"]')!.value).toBe('240');
    page().querySelector<HTMLSelectElement>('[name="method"]')!.value = 'Chèque';
    button('Noter le paiement').click();
    await settle();
    const request = http.expectOne('/api/admin/bookings/1/payments');
    expect(request.request.body).toMatchObject({ amount: 24_000, method: 'Chèque' });
    request.flush({ status: 'acompte_recu' });
    await settle();
    http.expectOne('/api/admin/bookings/1').flush({
      ...DETAIL,
      status: 'acompte_recu',
      amounts: { ...DETAIL.amounts, paid: 24_000 },
    });
    await settle();
    expect(page().querySelector('[data-testid="status"]')!.textContent).toContain('Acompte reçu');
  });

  it('F007-S11 cancels after confirmation, with a reason', async () => {
    await openDetail();
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
    button('Annuler la réservation').click();
    await settle();
    expect(page().querySelector('[role="alert"]')!.textContent).toContain('motif');
    const reason = page().querySelector<HTMLInputElement>('[name="reason"]')!;
    reason.value = 'Fausse réservation';
    button('Annuler la réservation').click();
    await settle();
    expect(confirm).toHaveBeenCalled();
    const request = http.expectOne('/api/admin/bookings/1/cancel');
    expect(request.request.body).toEqual({ reason: 'Fausse réservation' });
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    http.expectOne('/api/admin/bookings/1').flush({
      ...DETAIL,
      status: 'annulee',
      cancel_reason: 'Fausse réservation',
    });
    await settle();
    expect(page().querySelector('[data-testid="status"]')!.textContent).toContain('Annulée');
    confirm.mockRestore();
  });
});

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
