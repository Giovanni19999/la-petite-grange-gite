import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { smsCount } from './data-access/sms-count';
import { MessageTemplate } from './data-access/templates.api';
import { TemplatesPage } from './templates.page';

const TEMPLATES: MessageTemplate[] = [
  {
    kind: 'booking_confirmed',
    label: 'Réservation confirmée',
    recipient: 'guest',
    variables: [{ name: 'voyageur.prenom', label: 'Prénom du voyageur' }],
    channels: [
      {
        channel: 'email',
        texts: [
          { lang: 'fr', subject: 'Confirmée', body: '<p>Bonjour</p>', changed: false },
          { lang: 'en', subject: 'Confirmed', body: '<p>Hello</p>', changed: false },
        ],
      },
      {
        channel: 'sms',
        texts: [
          { lang: 'fr', subject: '', body: 'Réservation confirmée', changed: true },
          { lang: 'en', subject: '', body: '', changed: false },
        ],
      },
    ],
  },
];

describe('F007 SMS counter', () => {
  it('counts GSM-7 and Unicode SMS', () => {
    expect(smsCount('')).toEqual({ length: 0, unicode: false, segments: 0 });
    expect(smsCount('a'.repeat(160))).toEqual({ length: 160, unicode: false, segments: 1 });
    expect(smsCount('a'.repeat(161)).segments).toBe(2);
    expect(smsCount('€').length).toBe(2);
    // "é" is GSM-7, "ê" is not.
    expect(smsCount('é').unicode).toBe(false);
    const unicode = smsCount(`ê${'a'.repeat(169)}`);
    expect(unicode).toEqual({ length: 170, unicode: true, segments: 3 });
  });
});

describe('F007 templates page', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([{ path: 'admin/modeles', component: TemplatesPage }]),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
    await harness.navigateByUrl('/admin/modeles');
    http.expectOne('/api/admin/templates').flush(TEMPLATES);
    await settle();
  });

  afterEach(() => http.verify());

  const page = () => harness.routeNativeElement as HTMLElement;
  const button = (text: string) =>
    Array.from(page().querySelectorAll<HTMLButtonElement>('button')).find(
      (candidate) => candidate.textContent?.trim() === text,
    )!;

  it('F007-S12 saves an SMS with a variable and shows the refusals of the API', async () => {
    button('SMS').click();
    await settle();
    const sms = page().querySelector<HTMLTextAreaElement>('[name="sms"]')!;
    expect(sms.value).toBe('Réservation confirmée');
    button('Prénom du voyageur').click();
    await settle();
    expect(sms.value).toBe('Réservation confirmée {{ voyageur.prenom }}');
    expect(page().querySelector('[data-testid="sms-count"]')!.textContent).toContain('1 SMS');
    expect(button("Rétablir le texte d'origine")).toBeDefined();

    button('Valider').click();
    await settle();
    const request = http.expectOne('/api/admin/templates/booking_confirmed/sms/fr');
    expect(request.request.body).toEqual({
      subject: '',
      body: 'Réservation confirmée {{ voyageur.prenom }}',
    });
    request.flush(
      {
        error: {
          code: 'invalid_content',
          message: 'Contenu invalide',
          details: ['variable inconnue : voyageur.prenon'],
        },
      },
      { status: 422, statusText: 'Unprocessable Entity' },
    );
    await settle();
    expect(page().querySelector('[role="alert"]')!.textContent).toContain(
      'variable inconnue : voyageur.prenon',
    );
  });

  it('F007-S13 switches to the English text and previews it', async () => {
    button('Anglais').click();
    await settle();
    expect(page().querySelector<HTMLInputElement>('[name="subject"]')!.value).toBe('Confirmed');
    button('Aperçu').click();
    await settle();
    const request = http.expectOne('/api/admin/templates/preview');
    expect(request.request.body).toEqual({
      kind: 'booking_confirmed',
      channel: 'email',
      lang: 'en',
      subject: 'Confirmed',
      body: '<p>Hello</p>',
    });
    request.flush({ subject: 'Confirmed', html: '<p>Hello</p>', text: 'Hello' });
    await settle();
    expect(page().querySelector('[data-testid="preview"] iframe')).not.toBeNull();
  });
});

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
