import { DatePipe } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { firstValueFrom } from 'rxjs';

import { Choice, ChoiceGroup } from '../../../shared/choice/choice-group';
import { ApiErrorBody } from '../pages/data-access/admin-pages.models';
import {
  MessageStatus,
  MessagingApi,
  MessagingSettings,
  MessagingState,
  SentMessage,
  SmtpSecurity,
  TestResult,
} from './data-access/messaging.api';

/** The form as typed. */
export interface MessagingDraft {
  host: string;
  port: string;
  security: SmtpSecurity;
  username: string;
  fromAddress: string;
  fromName: string;
  replyTo: string;
  smsEnabled: boolean;
  smsSender: string;
  /** One per line. */
  ownerEmails: string;
  ownerPhones: string;
  maxAttempts: string;
  firstRetry: string;
}

const SECURITIES: readonly Choice<SmtpSecurity>[] = [
  { value: 'starttls', label: 'STARTTLS (port 587)' },
  { value: 'tls', label: 'TLS (port 465)' },
  { value: 'none', label: 'Aucune (test local)' },
];

const STATUS_LABELS: Record<MessageStatus, string> = {
  pending: 'En attente',
  sending: "En cours d'envoi",
  sent: 'Envoyé',
  failed: 'Échec',
  to_check: 'À vérifier',
  disabled: 'SMS désactivés',
};

export function draftOf(settings: MessagingSettings): MessagingDraft {
  return {
    host: settings.smtp.host,
    port: String(settings.smtp.port),
    security: settings.smtp.security,
    username: settings.smtp.username,
    fromAddress: settings.smtp.from_address,
    fromName: settings.smtp.from_name,
    replyTo: settings.smtp.reply_to ?? '',
    smsEnabled: settings.sms_enabled,
    smsSender: settings.sms_sender,
    ownerEmails: settings.owner_emails.join('\n'),
    ownerPhones: settings.owner_phones.join('\n'),
    maxAttempts: String(settings.max_attempts),
    firstRetry: String(settings.first_retry_minutes),
  };
}

function whole(text: string): number | null {
  return /^\d+$/.test(text.trim()) ? Number(text.trim()) : null;
}

function lines(text: string): string[] {
  return text
    .split(/[\n,;]/)
    .map((line) => line.trim())
    .filter((line) => line !== '');
}

/** The form → the settings, or the problems to fix (the API checks every rule again). */
export function settingsFrom(draft: MessagingDraft): {
  settings: MessagingSettings | null;
  errors: string[];
} {
  const errors: string[] = [];
  const port = whole(draft.port);
  if (port === null || port < 1 || port > 65535) {
    errors.push('Port : un nombre entre 1 et 65535.');
  }
  const maxAttempts = whole(draft.maxAttempts);
  const firstRetry = whole(draft.firstRetry);
  if (maxAttempts === null || firstRetry === null) {
    errors.push('Nouveaux essais : des nombres entiers.');
  }
  if (errors.length > 0) {
    return { settings: null, errors };
  }
  return {
    errors,
    settings: {
      smtp: {
        host: draft.host.trim(),
        port: port!,
        security: draft.security,
        username: draft.username.trim(),
        from_address: draft.fromAddress.trim(),
        from_name: draft.fromName.trim(),
        reply_to: draft.replyTo.trim() || null,
      },
      sms_enabled: draft.smsEnabled,
      sms_sender: draft.smsSender.trim(),
      owner_emails: lines(draft.ownerEmails),
      owner_phones: lines(draft.ownerPhones),
      max_attempts: maxAttempts!,
      first_retry_minutes: firstRetry!,
    },
  };
}

/** "Envoi des messages" (F025): SMTP server, SMS, notification recipients, tests and log. */
@Component({
  selector: 'app-messaging-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [DatePipe, RouterLink, HlmButtonImports, ChoiceGroup],
  template: `
    <section class="mx-auto flex max-w-3xl flex-col gap-6 px-4 py-8">
      <div>
        <a routerLink="/admin" class="text-sm text-muted-foreground hover:underline">
          ← Espace administrateur
        </a>
        <h1 class="text-2xl font-semibold">Envoi des messages</h1>
        <p class="mt-1 text-sm text-muted-foreground">
          Les e-mails et SMS envoyés par le site (confirmations de réservation…) partent par le
          serveur et le prestataire réglés ici.
        </p>
      </div>

      @if (loadError()) {
        <p class="text-destructive" role="alert">Impossible de charger les réglages.</p>
      }

      @if (state(); as s) {
        <ul class="flex flex-col gap-1 rounded-lg border p-4 text-sm" data-testid="server-status">
          <li>
            Mot de passe SMTP :
            @if (s.smtp_password_set) {
              <strong>défini</strong> dans la configuration du serveur.
            } @else {
              <strong>non défini</strong> (à ajouter dans la configuration du serveur si le compte
              en demande un).
            }
          </li>
          <li>
            Prestataire SMS :
            @if (s.sms_provider) {
              <strong>Brevo configuré</strong>.
            } @else {
              <strong>aucun</strong> : les SMS ne sont pas envoyés tant que la clé Brevo n'est pas
              ajoutée dans la configuration du serveur.
            }
          </li>
        </ul>
      }

      @if (draft(); as d) {
        <form class="flex flex-col gap-6" (submit)="$event.preventDefault(); save()">
          <fieldset class="flex flex-col gap-3 rounded-lg border p-4">
            <legend class="px-1 font-semibold">Serveur d'e-mails (SMTP)</legend>
            <div class="flex flex-wrap gap-3">
              <label class="flex flex-1 flex-col gap-1 text-sm">
                Serveur
                <input
                  #host
                  class="h-9 rounded-md border border-input px-2"
                  name="host"
                  placeholder="smtp.exemple.fr"
                  [value]="d.host"
                  (input)="patch({ host: host.value })"
                />
              </label>
              <label class="flex flex-col gap-1 text-sm">
                Port
                <input
                  #port
                  class="h-9 w-24 rounded-md border border-input px-2 text-right"
                  inputmode="numeric"
                  name="port"
                  [value]="d.port"
                  (input)="patch({ port: port.value })"
                />
              </label>
            </div>
            <app-choice-group
              legend="Sécurité de la connexion"
              [choices]="securities"
              [value]="d.security"
              (changed)="patch({ security: $event })"
            />
            @if (d.security === 'none') {
              <p class="text-sm text-amber-900" data-testid="insecure">
                Sans sécurité, le mot de passe et les messages passent en clair : à réserver à un
                serveur de test sur cet ordinateur.
              </p>
            }
            <label class="flex flex-col gap-1 text-sm">
              Identifiant (vide si le serveur n'en demande pas)
              <input
                #username
                class="h-9 rounded-md border border-input px-2"
                name="username"
                autocomplete="off"
                [value]="d.username"
                (input)="patch({ username: username.value })"
              />
            </label>
            <div class="flex flex-wrap gap-3">
              <label class="flex flex-1 flex-col gap-1 text-sm">
                Adresse d'expédition
                <input
                  #fromAddress
                  class="h-9 rounded-md border border-input px-2"
                  type="email"
                  name="fromAddress"
                  [value]="d.fromAddress"
                  (input)="patch({ fromAddress: fromAddress.value })"
                />
              </label>
              <label class="flex flex-1 flex-col gap-1 text-sm">
                Nom affiché
                <input
                  #fromName
                  class="h-9 rounded-md border border-input px-2"
                  name="fromName"
                  [value]="d.fromName"
                  (input)="patch({ fromName: fromName.value })"
                />
              </label>
            </div>
            <label class="flex flex-col gap-1 text-sm">
              Adresse de réponse (facultative)
              <input
                #replyTo
                class="h-9 rounded-md border border-input px-2"
                type="email"
                name="replyTo"
                [value]="d.replyTo"
                (input)="patch({ replyTo: replyTo.value })"
              />
            </label>
          </fieldset>

          <fieldset class="flex flex-col gap-3 rounded-lg border p-4">
            <legend class="px-1 font-semibold">SMS</legend>
            <label class="flex items-center gap-2 text-sm">
              <input
                #smsEnabled
                type="checkbox"
                class="size-4 accent-primary"
                name="smsEnabled"
                [checked]="d.smsEnabled"
                (change)="patch({ smsEnabled: smsEnabled.checked })"
              />
              Envoyer des SMS
            </label>
            <label class="flex flex-col gap-1 text-sm">
              Nom de l'expéditeur (3 à 11 lettres ou chiffres, affiché sur le téléphone)
              <input
                #smsSender
                class="h-9 w-44 rounded-md border border-input px-2"
                name="smsSender"
                maxlength="11"
                [value]="d.smsSender"
                (input)="patch({ smsSender: smsSender.value })"
              />
            </label>
          </fieldset>

          <fieldset class="flex flex-col gap-3 rounded-lg border p-4">
            <legend class="px-1 font-semibold">Destinataires des notifications</legend>
            <p class="text-sm text-muted-foreground">
              Ils reçoivent les nouvelles réservations et les messages de test. Un par ligne.
            </p>
            <label class="flex flex-col gap-1 text-sm">
              Adresses e-mail
              <textarea
                #ownerEmails
                class="min-h-16 rounded-md border border-input p-2"
                name="ownerEmails"
                [value]="d.ownerEmails"
                (input)="patch({ ownerEmails: ownerEmails.value })"
              ></textarea>
            </label>
            <label class="flex flex-col gap-1 text-sm">
              Téléphones (format international, par exemple +33612345678)
              <textarea
                #ownerPhones
                class="min-h-16 rounded-md border border-input p-2"
                name="ownerPhones"
                [value]="d.ownerPhones"
                (input)="patch({ ownerPhones: ownerPhones.value })"
              ></textarea>
            </label>
          </fieldset>

          <fieldset class="flex flex-col gap-3 rounded-lg border p-4">
            <legend class="px-1 font-semibold">En cas d'échec</legend>
            <div class="flex flex-wrap gap-3">
              <label class="flex flex-col gap-1 text-sm">
                Nombre d'essais
                <input
                  #maxAttempts
                  class="h-9 w-24 rounded-md border border-input px-2 text-right"
                  inputmode="numeric"
                  name="maxAttempts"
                  [value]="d.maxAttempts"
                  (input)="patch({ maxAttempts: maxAttempts.value })"
                />
              </label>
              <label class="flex flex-col gap-1 text-sm">
                Premier délai (minutes, doublé à chaque essai)
                <input
                  #firstRetry
                  class="h-9 w-24 rounded-md border border-input px-2 text-right"
                  inputmode="numeric"
                  name="firstRetry"
                  [value]="d.firstRetry"
                  (input)="patch({ firstRetry: firstRetry.value })"
                />
              </label>
            </div>
          </fieldset>

          @if (errors().length > 0) {
            <ul class="list-inside list-disc text-sm text-destructive" role="alert">
              @for (error of errors(); track error) {
                <li>{{ error }}</li>
              }
            </ul>
          }
          <div class="flex flex-wrap items-center gap-3">
            <button hlmBtn type="submit" [disabled]="busy() || !dirty()">Valider</button>
            @if (dirty()) {
              <button hlmBtn variant="outline" type="button" (click)="reset()">Annuler</button>
              <span class="text-xs text-amber-900" data-testid="unsaved">
                Modifications non enregistrées
              </span>
            } @else if (saved()) {
              <span class="text-sm text-muted-foreground" role="status">Réglages enregistrés.</span>
            }
          </div>
        </form>

        <section class="flex flex-col gap-3 rounded-lg border p-4" aria-labelledby="tests-title">
          <h2 id="tests-title" class="font-semibold">Essayer l'envoi</h2>
          @if (dirty()) {
            <p class="text-sm text-muted-foreground">Validez d'abord les réglages modifiés.</p>
          }
          <div class="flex flex-wrap gap-2">
            <button
              hlmBtn
              variant="outline"
              type="button"
              [disabled]="busy() || dirty()"
              (click)="test('email')"
            >
              Envoyer un e-mail de test
            </button>
            <button
              hlmBtn
              variant="outline"
              type="button"
              [disabled]="busy() || dirty()"
              (click)="test('sms')"
            >
              Envoyer un SMS de test
            </button>
          </div>
          @if (testResults(); as results) {
            <ul class="flex flex-col gap-1 text-sm" data-testid="test-results" role="status">
              @for (result of results; track result.recipient) {
                <li [class.text-destructive]="!result.sent">
                  {{ result.recipient }} :
                  {{ result.sent ? 'envoyé' : 'échec, ' + result.error }}
                </li>
              }
            </ul>
          }
        </section>

        <section class="flex flex-col gap-3" aria-labelledby="log-title">
          <div class="flex items-center justify-between gap-2">
            <h2 id="log-title" class="font-semibold">Derniers messages</h2>
            <button hlmBtn variant="ghost" size="sm" type="button" (click)="loadMessages()">
              Actualiser
            </button>
          </div>
          <ul class="flex flex-col divide-y rounded-lg border text-sm" data-testid="messages">
            @for (message of messages(); track message.id) {
              <li
                class="flex flex-wrap items-center gap-x-3 gap-y-1 p-3"
                [attr.data-message]="message.id"
              >
                <span class="text-muted-foreground">
                  {{ message.created_at | date: 'dd/MM/yyyy HH:mm' }}
                </span>
                <span>{{ message.channel === 'email' ? 'E-mail' : 'SMS' }}</span>
                <span class="font-medium">{{ message.recipient }}</span>
                <span class="min-w-0 flex-1 truncate">{{ message.summary }}</span>
                <span
                  class="rounded px-1.5 py-0.5 text-xs"
                  [class]="statusClass(message.status)"
                  data-testid="status"
                >
                  {{ statusLabel(message) }}
                </span>
                @if (message.error && message.status !== 'sent') {
                  <span class="w-full text-xs text-destructive">{{ message.error }}</span>
                }
                @if (resendable(message)) {
                  <button
                    hlmBtn
                    variant="outline"
                    size="sm"
                    type="button"
                    [disabled]="busy()"
                    (click)="resend(message)"
                  >
                    Renvoyer
                  </button>
                }
              </li>
            } @empty {
              <li class="p-3 text-muted-foreground">Aucun message envoyé pour l'instant.</li>
            }
          </ul>
        </section>
      }
    </section>
  `,
})
export class MessagingPage {
  private readonly api = inject(MessagingApi);

  protected readonly securities = SECURITIES;

  protected readonly state = signal<MessagingState | null>(null);
  protected readonly draft = signal<MessagingDraft | null>(null);
  protected readonly messages = signal<SentMessage[]>([]);
  protected readonly testResults = signal<TestResult[] | null>(null);
  protected readonly loadError = signal(false);
  protected readonly busy = signal(false);
  protected readonly saved = signal(false);
  protected readonly errors = signal<string[]>([]);

  protected readonly dirty = computed(() => {
    const state = this.state();
    const draft = this.draft();
    return (
      state !== null &&
      draft !== null &&
      JSON.stringify(draftOf(state.settings)) !== JSON.stringify(draft)
    );
  });

  constructor() {
    void this.load();
    void this.loadMessages();
  }

  protected patch(changes: Partial<MessagingDraft>): void {
    this.draft.update((draft) => (draft ? { ...draft, ...changes } : draft));
    this.saved.set(false);
  }

  protected reset(): void {
    const state = this.state();
    if (state) {
      this.draft.set(draftOf(state.settings));
    }
    this.errors.set([]);
  }

  protected async save(): Promise<void> {
    const draft = this.draft();
    if (!draft) {
      return;
    }
    const { settings, errors } = settingsFrom(draft);
    this.errors.set(errors);
    if (!settings) {
      return;
    }
    await this.run(async () => {
      await firstValueFrom(this.api.save(settings));
      await this.load();
      this.saved.set(true);
    });
  }

  protected async test(channel: 'email' | 'sms'): Promise<void> {
    this.testResults.set(null);
    await this.run(async () => {
      this.testResults.set(
        await firstValueFrom(channel === 'email' ? this.api.testEmail() : this.api.testSms()),
      );
    });
  }

  protected async resend(message: SentMessage): Promise<void> {
    await this.run(async () => {
      await firstValueFrom(this.api.resend(message.id));
      await this.loadMessages();
    });
  }

  protected async loadMessages(): Promise<void> {
    try {
      this.messages.set(await firstValueFrom(this.api.messages()));
    } catch {
      this.messages.set([]);
    }
  }

  protected statusLabel(message: SentMessage): string {
    const label = STATUS_LABELS[message.status];
    return message.status === 'pending' && message.attempts > 0
      ? `${label} (essai ${message.attempts + 1})`
      : label;
  }

  protected statusClass(status: MessageStatus): string {
    switch (status) {
      case 'sent':
        return 'bg-green-100 text-green-900';
      case 'failed':
      case 'to_check':
        return 'bg-red-100 text-red-900';
      default:
        return 'bg-muted text-muted-foreground';
    }
  }

  protected resendable(message: SentMessage): boolean {
    return ['failed', 'to_check', 'disabled'].includes(message.status);
  }

  private async run(action: () => Promise<void>): Promise<void> {
    this.busy.set(true);
    try {
      await action();
    } catch (error) {
      this.errors.set(errorMessages(error));
    } finally {
      this.busy.set(false);
    }
  }

  private async load(): Promise<void> {
    try {
      const state = await firstValueFrom(this.api.state());
      this.state.set(state);
      this.draft.set(draftOf(state.settings));
      this.loadError.set(false);
    } catch {
      this.loadError.set(true);
    }
  }
}

function errorMessages(error: unknown): string[] {
  const body = error instanceof HttpErrorResponse ? (error.error as ApiErrorBody | null) : null;
  const details = body?.error?.details ?? [];
  if (details.length > 0) {
    return details;
  }
  return [body?.error?.message ?? 'Une erreur est survenue, réessayez plus tard.'];
}
