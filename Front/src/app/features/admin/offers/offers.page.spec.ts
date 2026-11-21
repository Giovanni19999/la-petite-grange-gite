import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import {
  EMPTY_OPTION,
  EMPTY_PROMOTION,
  optionFrom,
  promotionFrom,
} from './data-access/offer-drafts';
import { Offers } from './data-access/offers.api';
import { OffersPage } from './offers.page';

const URL = '/api/admin/offers';

const OFFERS: Offers = {
  promotions: [
    {
      id: 3,
      name: { fr: 'Semaine', en: 'Week' },
      discount: { kind: 'percent', bp: 1000 },
      min_nights: 7,
      max_nights: null,
      valid_from: null,
      valid_until: null,
      active: true,
    },
  ],
  options: [
    {
      id: 5,
      name: { fr: 'Lit bébé', en: '' },
      description: { fr: '', en: '' },
      unit: 'stay',
      price: 0,
      min_nights: null,
      max_nights: null,
      position: 0,
      active: true,
    },
  ],
};

describe('F026 offer drafts', () => {
  it('turns a promotion form into a promotion, bounds and dates optional', () => {
    expect(
      promotionFrom({ ...EMPTY_PROMOTION, nameFr: ' Semaine ', value: '10', min: '7' }).promotion,
    ).toEqual({
      name: { fr: 'Semaine', en: '' },
      discount: { kind: 'percent', bp: 1000 },
      min_nights: 7,
      max_nights: null,
      valid_from: null,
      valid_until: null,
      active: true,
    });
    expect(
      promotionFrom({
        ...EMPTY_PROMOTION,
        nameFr: 'Court',
        kind: 'per_night',
        value: '5',
        max: '3',
      }).promotion?.discount,
    ).toEqual({ kind: 'per_night', amount: 500 });
  });

  it('lists the mistakes of a form', () => {
    expect(promotionFrom({ ...EMPTY_PROMOTION, value: '0', min: '7', max: '3' }).errors).toEqual([
      'Le nom en français est obligatoire.',
      'Le minimum de nuits ne peut pas dépasser le maximum.',
      'Remise : un pourcentage entre 0 et 100.',
    ]);
    expect(optionFrom({ ...EMPTY_OPTION, nameFr: 'Linge', price: 'abc', min: 'x' }).errors).toEqual(
      [
        'Prix : un montant en euros (0 si offert).',
        'Nuits minimum et maximum : un nombre entier, ou vide.',
      ],
    );
  });

  it('an option at 0 € is offered, priced per guest and night', () => {
    expect(
      optionFrom({ ...EMPTY_OPTION, nameFr: 'Petit-déjeuner', unit: 'guest_night', price: '8' })
        .option,
    ).toMatchObject({ unit: 'guest_night', price: 800, min_nights: null });
  });
});

describe('F026 promotions and options page', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([{ path: 'admin/promotions', component: OffersPage }]),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
    await harness.navigateByUrl('/admin/promotions');
    http.expectOne(URL).flush(OFFERS);
    await settle();
  });

  afterEach(() => http.verify());

  const page = () => harness.routeNativeElement as HTMLElement;
  const button = (text: string, scope: ParentNode = page()) =>
    Array.from(scope.querySelectorAll<HTMLButtonElement>('button')).find(
      (candidate) => candidate.textContent?.trim() === text,
    )!;
  const form = () => page().querySelector<HTMLFormElement>('form')!;
  async function type(name: string, value: string): Promise<void> {
    const field = form().querySelector<HTMLInputElement>(`[name="${name}"]`)!;
    field.value = value;
    field.dispatchEvent(new Event('input'));
    await settle();
  }

  it('lists the promotions and options in plain words', () => {
    expect(page().querySelector('[data-testid="promotions"]')!.textContent).toContain(
      'Semaine · −10 % · dès 7 nuits',
    );
    expect(page().querySelector('[data-testid="options"]')!.textContent).toContain(
      'Lit bébé · offert · toute durée',
    );
  });

  it('F026-S01 creates a promotion from 7 nights', async () => {
    button('Nouvelle promotion').click();
    await settle();
    await type('nameFr', 'Long séjour');
    await type('value', '10');
    await type('min', '7');
    button('Valider', form()).click();
    await settle();
    const request = http.expectOne(`${URL}/promotions`);
    expect(request.request.method).toBe('POST');
    expect(request.request.body).toMatchObject({
      name: { fr: 'Long séjour', en: '' },
      discount: { kind: 'percent', bp: 1000 },
      min_nights: 7,
      max_nights: null,
    });
    request.flush({ id: 4 });
    await settle();
    http.expectOne(URL).flush(OFFERS);
    await settle();
    expect(page().querySelector('form')).toBeNull();
  });

  it('F026-S04 edits an option and shows the API refusals', async () => {
    button('Modifier', page().querySelector('[data-testid="options"]')!).click();
    await settle();
    page()
      .querySelector<HTMLInputElement>('input[type="radio"][value="guest"]')!
      .dispatchEvent(new Event('change'));
    await settle();
    await type('price', '15');
    await type('min', '3');
    button('Valider', form()).click();
    await settle();
    const request = http.expectOne(`${URL}/options/5`);
    expect(request.request.body).toMatchObject({ unit: 'guest', price: 1500, min_nights: 3 });
    request.flush(
      {
        error: {
          code: 'invalid',
          message: 'Contenu invalide',
          details: ['le nom fait au plus 80 caractères'],
        },
      },
      { status: 422, statusText: 'Unprocessable Entity' },
    );
    await settle();
    expect(form().textContent).toContain('le nom fait au plus 80 caractères');
  });

  it('deletes after confirmation', async () => {
    button('Supprimer', page().querySelector('[data-testid="promotions"]')!).click();
    await settle();
    (document.querySelector('[data-slot="alert-dialog-action"]') as HTMLButtonElement).click();
    await settle();
    const request = http.expectOne(`${URL}/promotions/3`);
    expect(request.request.method).toBe('DELETE');
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    http.expectOne(URL).flush({ ...OFFERS, promotions: [] });
    await settle();
    expect(page().textContent).toContain('Aucune promotion.');
  });
});

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
