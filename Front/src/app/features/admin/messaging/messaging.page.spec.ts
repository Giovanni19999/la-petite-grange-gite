import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { MessagingState, SentMessage } from './data-access/messaging.api';
import { MessagingPage, draftOf, settingsFrom } from './messaging.page';

const URL = '/api/admin/messaging';

const STATE: MessagingState = {
  settings: {
    smtp: {
      host: 'smtp.example.test',
      port: 587,
      security: 'starttls',
      username: 'site',
      from_address: 'site@example.test',
      from_name: 'Le gîte',
      reply_to: null,
    },
    sms_enabled: false,
    sms_sender: 'LaGrange',
    owner_emails: ['proprio@example.test'],
    owner_phones: ['+33612345678'],
    max_attempts: 5,
    first_retry_minutes: 5,
  },
  smtp_password_set: true,
  sms_provider: false,
};

const MESSAGES: SentMessage[] = [
  {
    id: 7,
    channel: 'email',
    recipient: 'guest@example.test',
    summary: 'Réservation confirmée',
    status: 'failed',
    attempts: 5,
    error: 'serveur injoignable',
    created_at: '2027-03-01T10:00:00Z',
    sent_at: null,
  },
  {
    id: 6,
    channel: 'sms',
    recipient: '+33612345678',
    summary: 'Nouvelle réservation',
    status: 'pending',
    attempts: 1,
    error: 'serveur injoignable',
    created_at: '2027-03-01T09:00:00Z',
    sent_at: null,
  },
];

describe('F025 messaging settings form', () => {
  it('turns the form into settings, one recipient per line', () => {
    const draft = {
      ...draftOf(STATE.settings),
      ownerEmails: 'a@example.test\n\n b@example.test ',
      replyTo: ' ',
    };
    expect(settingsFrom(draft).settings).toMatchObject({
      owner_emails: ['a@example.test', 'b@example.test'],
      smtp: { reply_to: null, port: 587 },
    });
    expect(settingsFrom({ ...draft, port: '99999', maxAttempts: 'x' }).errors).toEqual([
      'Port : un nombre entre 1 et 65535.',
      'Nouveaux essais : des nombres entiers.',
    ]);
  });
});

describe('F025 messaging page', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([{ path: 'admin/messages', component: MessagingPage }]),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
    await harness.navigateByUrl('/admin/messages');
    http.expectOne(URL).flush(STATE);
    http.expectOne(`${URL}/messages`).flush(MESSAGES);
    await settle();
  });

  afterEach(() => http.verify());

  const page = () => harness.routeNativeElement as HTMLElement;
  const button = (text: string) =>
    Array.from(page().querySelectorAll<HTMLButtonElement>('button')).find(
      (candidate) => candidate.textContent?.trim() === text,
    )!;
  async function type(name: string, value: string): Promise<void> {
    const field = page().querySelector<HTMLInputElement | HTMLTextAreaElement>(`[name="${name}"]`)!;
    field.value = value;
    field.dispatchEvent(new Event('input'));
    await settle();
  }

  it('shows what is set on the server without any secret', () => {
    const status = page().querySelector('[data-testid="server-status"]')!.textContent!;
    expect(status).toContain('défini');
    expect(status).toContain('aucun');
  });

  it('F025-S07 saves the settings and shows the refusals of the API', async () => {
    await type('ownerPhones', '+33612345678\n0612');
    await type('smsSender', 'LaGrange');
    button('Valider').click();
    await settle();
    const request = http.expectOne(URL);
    expect(request.request.method).toBe('PUT');
    expect(request.request.body.owner_phones).toEqual(['+33612345678', '0612']);
    request.flush(
      {
        error: {
          code: 'invalid',
          message: 'Contenu invalide',
          details: ['numéro de notification invalide : 0612'],
        },
      },
      { status: 422, statusText: 'Unprocessable Entity' },
    );
    await settle();
    expect(page().querySelector('[role="alert"]')!.textContent).toContain(
      'numéro de notification invalide : 0612',
    );
  });

  it('warns when the connection has no security', async () => {
    page()
      .querySelector<HTMLInputElement>('input[type="radio"][value="none"]')!
      .dispatchEvent(new Event('change'));
    await settle();
    expect(page().querySelector('[data-testid="insecure"]')).not.toBeNull();
  });

  it('F025-S03 sends a test email and shows the error in French', async () => {
    button('Envoyer un e-mail de test').click();
    await settle();
    http.expectOne(`${URL}/test-email`).flush([
      {
        recipient: 'proprio@example.test',
        sent: false,
        error: 'authentification refusée par le serveur',
      },
    ]);
    await settle();
    expect(page().querySelector('[data-testid="test-results"]')!.textContent).toContain(
      'proprio@example.test : échec, authentification refusée par le serveur',
    );
  });

  it('F025-S02 lists the failed messages and sends one again', async () => {
    const list = page().querySelector('[data-testid="messages"]')!;
    const failed = list.querySelector('[data-message="7"]')!;
    expect(failed.textContent).toContain('Échec');
    expect(failed.textContent).toContain('serveur injoignable');
    expect(list.querySelector('[data-message="6"]')!.textContent).toContain('En attente (essai 2)');

    Array.from(failed.querySelectorAll('button'))
      .find((candidate) => candidate.textContent?.trim() === 'Renvoyer')!
      .click();
    await settle();
    const request = http.expectOne(`${URL}/messages/7/resend`);
    expect(request.request.method).toBe('POST');
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    http.expectOne(`${URL}/messages`).flush([{ ...MESSAGES[0], status: 'pending', attempts: 0 }]);
    await settle();
    expect(page().querySelector('[data-message="7"]')!.textContent).toContain('En attente');
  });
});

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
