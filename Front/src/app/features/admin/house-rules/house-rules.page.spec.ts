import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { HouseRules } from './data-access/house-rules.api';
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
};

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
    http
      .expectOne(URL)
      .flush(
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

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
