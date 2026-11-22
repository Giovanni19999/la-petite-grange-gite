import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { HouseRules } from './data-access/house-rules.api';
import { PaymentInfo } from './data-access/payment-info.api';
import { HouseRulesPage } from './house-rules.page';

const URL = '/api/admin/house-rules';

const SEED: HouseRules = {
  capacity: 5,
  min_delay_days: 1,
  arrival_weekdays: [1, 2, 3, 4, 5, 6, 7],
  departure_weekdays: [1, 2, 3, 4, 5, 6, 7],
  deposit: { mode: 'percentage', percent: 50 },
  balance_days: 30,
  tourist_tax: { per_adult_night: 0, additional_bp: 0, exempt_under: 18 },
  horizon: { months: 24, until: null },
  arrival_from: 960,
  departure_before: 600,
  pets_allowed: false,
  deposit_alert_days: 7,
  terms_page: null,
};

const PAYMENT: PaymentInfo = {
  instructions: { fr: '<p>Virement</p>', en: '' },
  methods: ['Virement', 'Chèque'],
  document: null,
  provider: 'none',
};

const PAGES = [
  {
    id: 2,
    is_home: false,
    title: 'Conditions de location',
    slug: 'conditions',
    draft_version: 1,
    published_at: '2027-01-01T00:00:00Z',
    unpublished_changes: false,
  },
];

describe('F004 house rules page', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([{ path: 'admin/regles', component: HouseRulesPage }]),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
    await harness.navigateByUrl('/admin/regles');
    http.expectOne(URL).flush(SEED);
    http.expectOne('/api/admin/pages').flush(PAGES);
    http.expectOne('/api/admin/payment-info').flush(PAYMENT);
    await settle();
  });

  afterEach(() => http.verify());

  const page = () => harness.routeNativeElement as HTMLElement;
  const field = (name: string) => page().querySelector<HTMLInputElement>(`[name="${name}"]`)!;
  const button = (text: string) =>
    Array.from(page().querySelectorAll<HTMLButtonElement>('button')).find(
      (candidate) => candidate.textContent?.trim() === text,
    )!;
  async function type(name: string, value: string): Promise<void> {
    field(name).value = value;
    field(name).dispatchEvent(new Event('input'));
    await settle();
  }
  const day = (kind: 'arrivals' | 'departures', name: string) =>
    page().querySelector<HTMLInputElement>(
      `[data-weekdays="${kind}"] input[aria-label$=": ${name}"]`,
    )!;

  it('F004-S09 shows the seeded rules and saves the changes', async () => {
    expect(field('capacity').value).toBe('5');
    expect(field('taxPerNight').value).toBe('0');
    expect(button('Valider').disabled).toBe(true);

    await type('capacity', '6');
    await type('taxPerNight', '1,70');
    await type('taxAdditional', '10');
    day('arrivals', 'mercredi').click();
    await settle();
    expect(page().querySelector('[data-testid="unsaved"]')).not.toBeNull();

    button('Valider').click();
    await settle();
    const request = http.expectOne(URL);
    expect(request.request.method).toBe('PUT');
    expect(request.request.body).toEqual({
      ...SEED,
      capacity: 6,
      arrival_weekdays: [1, 2, 4, 5, 6, 7],
      tourist_tax: { per_adult_night: 170, additional_bp: 1000, exempt_under: 18 },
    });
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    const saved = { ...SEED, capacity: 6 };
    http.expectOne(URL).flush(saved);
    await settle();
    expect(page().textContent).toContain('Règles enregistrées.');
  });

  it('F004-S09 a fixed deposit is sent in cents', async () => {
    page()
      .querySelector<HTMLInputElement>('input[type="radio"][value="fixed"]')!
      .dispatchEvent(new Event('change'));
    await settle();
    await type('depositAmount', '200');
    button('Valider').click();
    await settle();
    const request = http.expectOne(URL);
    expect(request.request.body.deposit).toEqual({ mode: 'fixed', amount: 20_000 });
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    http.expectOne(URL).flush(SEED);
    await settle();
  });

  it('F004-S09 shows the mistakes before calling the API, and the API refusals', async () => {
    await type('capacity', 'cinq');
    day('departures', 'lundi').click();
    for (const name of ['mardi', 'mercredi', 'jeudi', 'vendredi', 'samedi', 'dimanche']) {
      day('departures', name).click();
    }
    await settle();
    button('Valider').click();
    await settle();
    http.expectNone(URL);
    expect(page().textContent).toContain('Capacité : un nombre entier de personnes.');
    expect(page().textContent).toContain('Choisissez au moins un jour de départ.');

    await type('capacity', '80');
    day('departures', 'samedi').click();
    await settle();
    button('Valider').click();
    await settle();
    http.expectOne(URL).flush(
      {
        error: {
          code: 'invalid',
          message: 'Contenu invalide',
          details: ['capacité : de 1 à 50 personnes'],
        },
      },
      { status: 422, statusText: 'Unprocessable Entity' },
    );
    await settle();
    expect(page().textContent).toContain('capacité : de 1 à 50 personnes');
  });
});

describe('F007 house rules of the bookings', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([{ path: 'admin/regles', component: HouseRulesPage }]),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
    await harness.navigateByUrl('/admin/regles');
    http.expectOne(URL).flush(SEED);
    http.expectOne('/api/admin/pages').flush(PAGES);
    http.expectOne('/api/admin/payment-info').flush(PAYMENT);
    await settle();
  });

  afterEach(() => http.verify());

  const page = () => harness.routeNativeElement as HTMLElement;
  const buttons = (text: string) =>
    Array.from(page().querySelectorAll<HTMLButtonElement>('button')).filter(
      (candidate) => candidate.textContent?.trim() === text,
    );
  async function choose(name: string, value: string): Promise<void> {
    const select = page().querySelector<HTMLSelectElement>(`select[name="${name}"]`)!;
    select.value = value;
    select.dispatchEvent(new Event('change'));
    await settle();
  }

  it('F007 says booking is closed until a rental terms page is chosen, then saves it', async () => {
    expect(page().querySelector('[data-testid="booking-closed"]')).not.toBeNull();
    await choose('termsPage', '2');
    await choose('arrivalFrom', '1020');
    const alert = page().querySelector<HTMLInputElement>('[name="alertDays"]')!;
    alert.value = '10';
    alert.dispatchEvent(new Event('input'));
    page().querySelector<HTMLInputElement>('[name="pets"]')!.click();
    await settle();
    expect(page().querySelector('[data-testid="booking-closed"]')).toBeNull();
    buttons('Valider')[0].click();
    await settle();
    const request = http.expectOne(URL);
    expect(request.request.body).toEqual({
      ...SEED,
      terms_page: 2,
      arrival_from: 1020,
      deposit_alert_days: 10,
      pets_allowed: true,
    });
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    http.expectOne(URL).flush({ ...SEED, terms_page: 2 });
    await settle();
  });

  it('F007 saves the payment methods with their own button', async () => {
    const methods = page().querySelector<HTMLTextAreaElement>('[name="methods"]')!;
    expect(methods.value).toBe('Virement\nChèque');
    methods.value = 'Virement\n\n Chèques-vacances ';
    methods.dispatchEvent(new Event('input'));
    await settle();
    const [, paymentButton] = buttons('Valider');
    paymentButton.click();
    await settle();
    const request = http.expectOne('/api/admin/payment-info');
    expect(request.request.method).toBe('PUT');
    expect(request.request.body).toEqual({
      instructions: PAYMENT.instructions,
      methods: ['Virement', 'Chèques-vacances'],
    });
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    http
      .expectOne('/api/admin/payment-info')
      .flush({ ...PAYMENT, methods: ['Virement', 'Chèques-vacances'] });
    await settle();
    expect(page().textContent).toContain('Instructions enregistrées.');
    expect(page().querySelector('[data-testid="document"]')!.textContent).toContain(
      'Aucun document',
    );
  });
});

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
