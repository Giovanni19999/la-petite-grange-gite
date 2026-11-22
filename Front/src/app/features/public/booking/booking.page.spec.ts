import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter, withComponentInputBinding } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { Quote } from '../pages/data-access/availability.api';
import { BookingPage } from './booking.page';
import { BookingInfo, TrackedBooking } from './data-access/booking.api';
import { TrackingPage } from './tracking.page';

const INFO: BookingInfo = {
  open: true,
  capacity: 5,
  pets_allowed: false,
  arrival_from: 960,
  departure_before: 600,
  arrival_slots: [960, 990, 1020],
  instructions: '<p>Virement au RIB joint</p>',
  document: true,
  terms: { slug: 'conditions', title: 'Conditions' },
};

const QUOTE: Quote = {
  nights: 4,
  nights_total: 40_000,
  promotion: null,
  cleaning_fee: 12_000,
  options: [],
  options_total: 0,
  stay_total: 52_000,
  deposit: 26_000,
  balance: 26_000,
  balance_due: '2027-06-10',
  tourist_tax: 1_200,
};

const TRACKED: TrackedBooking = {
  reference: 'LPG-2027-0001',
  status: 'confirmee',
  lang: 'fr',
  arrival: '2027-07-10',
  departure: '2027-07-14',
  adults: 2,
  children: 0,
  first_name: 'Léa',
  arrival_time: 1080,
  arrival_slots: [960, 990, 1020, 1050, 1080, 1110],
  arrival_time_changeable: true,
  arrival_from: 960,
  departure_before: 600,
  pets_allowed: false,
  amounts: {
    nights: 4,
    nights_total: 40_000,
    promotion: null,
    discount: 0,
    cleaning_fee: 12_000,
    options: [],
    options_total: 0,
    stay_total: 52_000,
    deposit: 26_000,
    balance: 26_000,
    balance_due: '2027-06-10',
    tourist_tax: 1_200,
    paid: 0,
  },
  instructions: '<p>Virement</p>',
};

describe('F007 booking and tracking pages', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter(
          [
            { path: 'reserver', component: BookingPage, data: { lang: 'fr' } },
            { path: 'reservation/:token', component: TrackingPage, data: { lang: 'fr' } },
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
  async function openBooking(): Promise<void> {
    await harness.navigateByUrl(
      '/reserver?arrival=2027-07-10&departure=2027-07-14&adults=2&children=0',
    );
    await settle();
    http.expectOne((request) => request.url === '/api/booking-info').flush(INFO);
    await settle();
    const quote = http.expectOne('/api/quote');
    expect(quote.request.body).toEqual({
      arrival: '2027-07-10',
      departure: '2027-07-14',
      adults: 2,
      children: 0,
      options: [],
      lang: 'fr',
    });
    quote.flush(QUOTE);
    await settle();
  }
  async function type(name: string, value: string): Promise<void> {
    const field = page().querySelector<HTMLInputElement>(`[name="${name}"]`)!;
    field.value = value;
    field.dispatchEvent(new Event('input'));
    await settle();
  }

  async function fillGuest(): Promise<void> {
    for (const [name, value] of [
      ['first_name', 'Léa'],
      ['last_name', 'Martin'],
      ['email', 'lea@example.test'],
      ['phone', '+33612345678'],
      ['street', '1 rue des Lilas'],
      ['postcode', '75001'],
      ['town', 'Paris'],
    ]) {
      await type(name, value);
    }
  }

  it('F007-S01 shows the amounts, the rules and how to pay, then books', async () => {
    await openBooking();
    const amounts = page().querySelector('[data-testid="amounts"]')!.textContent!;
    expect(amounts).toMatch(/520/);
    expect(amounts).toMatch(/260/);
    const rules = page().querySelector('[data-testid="rules"]')!.textContent!;
    expect(rules).toContain('Arrivée à partir de 16:00');
    expect(rules).toContain('Virement au RIB joint');
    expect(rules).toContain('Le RIB est joint');

    await fillGuest();
    page().querySelector<HTMLInputElement>('[name="terms"]')!.click();
    await settle();
    page().querySelector<HTMLButtonElement>('button[type="submit"]')!.click();
    await settle();
    const request = http.expectOne('/api/bookings');
    expect(request.request.body).toMatchObject({
      arrival: '2027-07-10',
      adults: 2,
      lang: 'fr',
      arrival_time: 960,
      terms_accepted: true,
      website: '',
      guest: { first_name: 'Léa', country: 'France', phone: '+33612345678' },
    });
    request.flush(
      { reference: 'LPG-2027-0001', token: 'abc' },
      { status: 201, statusText: 'Created' },
    );
    await settle();
    http.expectOne('/api/bookings/abc').flush(TRACKED);
    await settle();
    expect(page().querySelector('[data-testid="confirmed"]')!.textContent).toContain(
      'LPG-2027-0001 est confirmée',
    );
  });

  it('F007-S10 shows the refusals of the API', async () => {
    await openBooking();
    await fillGuest();
    page().querySelector<HTMLButtonElement>('button[type="submit"]')!.click();
    await settle();
    http.expectOne('/api/bookings').flush(
      {
        error: {
          code: 'invalid_content',
          message: 'Contenu invalide',
          details: ['vous devez accepter les conditions de location'],
        },
      },
      { status: 422, statusText: 'Unprocessable Entity' },
    );
    await settle();
    expect(page().querySelector('[role="alert"]')!.textContent).toContain(
      'vous devez accepter les conditions de location',
    );
  });

  it('F007 says when booking is closed', async () => {
    await harness.navigateByUrl('/reserver?arrival=2027-07-10&departure=2027-07-14');
    await settle();
    http
      .expectOne((request) => request.url === '/api/booking-info')
      .flush({ ...INFO, open: false });
    await settle();
    expect(page().textContent).toContain('pas encore ouverte');
  });

  it('F007-S14 tracking page changes the arrival time', async () => {
    await harness.navigateByUrl('/reservation/abc');
    await settle();
    http.expectOne('/api/bookings/abc').flush(TRACKED);
    await settle();
    expect(page().querySelector('[data-testid="status"]')!.textContent).toContain(
      'acompte attendu',
    );
    const select = page().querySelector<HTMLSelectElement>('select[name="arrivalTime"]')!;
    select.value = '1110';
    page().querySelector<HTMLButtonElement>('button[type="submit"]')!.click();
    await settle();
    const request = http.expectOne('/api/bookings/abc/arrival-time');
    expect(request.request.body).toEqual({ arrival_time: 1110 });
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    http.expectOne('/api/bookings/abc').flush({ ...TRACKED, arrival_time: 1110 });
    await settle();
    expect(page().textContent).toContain('Heure d’arrivée enregistrée');
  });

  it('F007-S14 unknown token', async () => {
    await harness.navigateByUrl('/reservation/nope');
    await settle();
    http
      .expectOne('/api/bookings/nope')
      .flush(
        { error: { code: 'not_found', message: 'x' } },
        { status: 404, statusText: 'Not Found' },
      );
    await settle();
    expect(page().textContent).toContain('Réservation introuvable');
  });
});

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
