import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { PriceBook } from './data-access/prices.api';
import { PricesPage } from './prices.page';

const BASE = '/api/admin/prices';

const BOOK: PriceBook = {
  channels: [
    {
      code: 'direct',
      name: 'Site',
      commission_bp: 0,
      payment_fees_bp: 0,
      vat_on_commission: false,
      effective_bp: 0,
    },
    {
      code: 'airbnb',
      name: 'Airbnb',
      commission_bp: 1550,
      payment_fees_bp: 0,
      vat_on_commission: true,
      effective_bp: 1860,
    },
    {
      code: 'booking',
      name: 'Booking',
      commission_bp: 1500,
      payment_fees_bp: 140,
      vat_on_commission: true,
      effective_bp: 1968,
    },
  ],
  settings: { cleaning_fee: 0, default_min_nights: 2, commission_alert_points: 5 },
  defaults: { direct: { listed: 10_000, net: 10_000 } },
  default_typical_stays: {
    '2': { direct: [20_000, 20_000], airbnb: null, booking: null },
    '7': { direct: [70_000, 70_000], airbnb: null, booking: null },
  },
  ranges: [
    {
      id: 4,
      start: '2027-07-01',
      end: '2027-08-01',
      label: 'Juillet',
      min_nights: 3,
      prices: { direct: { listed: 12_000, net: 12_000 } },
      typical_stays: {},
    },
  ],
  incomplete: ['airbnb', 'booking'],
};

describe('F006 prices page', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([{ path: 'admin/tarifs', component: PricesPage }]),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
    await harness.navigateByUrl('/admin/tarifs');
    await flushBook();
  });

  afterEach(() => http.verify());

  async function flushBook(): Promise<void> {
    http.expectOne(BASE).flush(BOOK);
    await settle();
    http.expectOne((request) => request.url === `${BASE}/nights`).flush([]);
    await settle();
  }

  const page = () => harness.routeNativeElement as HTMLElement;
  const buttons = (text: string, scope: ParentNode = page()) =>
    Array.from(scope.querySelectorAll<HTMLButtonElement>('button')).filter(
      (candidate) => candidate.textContent?.trim() === text,
    );
  const defaults = () => page().querySelector<HTMLElement>('[data-testid="defaults"]')!;
  const field = (scope: ParentNode, channel: string, name: 'net' | 'listed') =>
    scope.querySelector<HTMLInputElement>(`tr[data-channel="${channel}"] [data-field="${name}"]`)!;
  async function type(input: HTMLInputElement, value: string): Promise<void> {
    input.value = value;
    input.dispatchEvent(new Event('input'));
    await settle();
  }
  const confirm = () =>
    (document.querySelector('[data-slot="alert-dialog-action"]') as HTMLButtonElement).click();

  it('F006-S01 shows the seeded prices and the incomplete channels', () => {
    expect(field(defaults(), 'direct', 'listed').value).toBe('100');
    expect(field(defaults(), 'booking', 'net').value).toBe('');
    expect(page().querySelector('[data-testid="incomplete"]')?.textContent).toContain(
      'Airbnb, Booking',
    );
  });

  it('F006 nothing is saved before "Valider"; it saves the whole default table at once', async () => {
    const validate = buttons('Valider', defaults())[0];
    expect(validate.disabled).toBe(true);

    await type(field(defaults(), 'booking', 'net'), '110');
    expect(field(defaults(), 'booking', 'listed').value).toBe('137');
    expect(field(defaults(), 'direct', 'listed').value).toBe('100');
    expect(defaults().querySelector('[data-testid="unsaved"]')).not.toBeNull();
    http.expectNone(`${BASE}/defaults`);

    validate.click();
    await settle();
    confirm();
    await settle();
    const request = http.expectOne(`${BASE}/defaults`);
    expect(request.request.method).toBe('PUT');
    expect(request.request.body).toEqual({
      direct: { listed: 10_000, net: 10_000 },
      airbnb: null,
      booking: { listed: 13_700, net: 11_000 },
    });
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    await flushBook();
  });

  it('F006-S02 a range is saved with its own "Valider", empty channels use the default', async () => {
    const range = page().querySelector<HTMLElement>('[data-range="4"]')!;
    const validate = buttons('Valider', range)[0];
    expect(validate.disabled).toBe(true);

    await type(field(range, 'airbnb', 'listed'), '150');
    expect(field(range, 'airbnb', 'net').value).toBe('122');
    expect(field(defaults(), 'airbnb', 'net').value).toBe('');

    validate.click();
    await settle();
    confirm();
    await settle();
    const request = http.expectOne(`${BASE}/ranges/4`);
    expect(request.request.method).toBe('PUT');
    expect(request.request.body).toEqual({
      start: '2027-07-01',
      end: '2027-08-01',
      label: 'Juillet',
      min_nights: 3,
      prices: { direct: { listed: 12_000, net: 12_000 }, airbnb: { listed: 15_000, net: 12_200 } },
    });
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    await flushBook();
  });

  it('F006 an emptied row is no longer unsaved', async () => {
    await type(field(defaults(), 'airbnb', 'listed'), '150');
    expect(defaults().querySelector('[data-testid="unsaved"]')).not.toBeNull();
    await type(field(defaults(), 'airbnb', 'listed'), '');
    expect(field(defaults(), 'airbnb', 'net').value).toBe('');
    expect(defaults().querySelector('[data-testid="unsaved"]')).toBeNull();
  });

  it('F006-S05 shows a half-filled row as an error without calling the API', async () => {
    await type(field(defaults(), 'airbnb', 'net'), '90');
    await type(field(defaults(), 'airbnb', 'listed'), 'abc');
    expect(field(defaults(), 'airbnb', 'net').value).toBe('90');
    buttons('Valider', defaults())[0].click();
    await settle();
    expect(defaults().textContent).toContain('Airbnb : remplissez les deux prix');
    expect(document.querySelector('[data-slot="alert-dialog-action"]')).toBeNull();
  });
});

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
