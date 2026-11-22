import { DatePipe } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectionStrategy, Component, inject, input, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { Observable, firstValueFrom } from 'rxjs';

import { formatLong, formatShort, todayInParis } from '../../../shared/dates/dates';
import { clockTime } from '../house-rules/house-rules.page';
import { PaymentInfoApi } from '../house-rules/data-access/payment-info.api';
import { ApiErrorBody } from '../pages/data-access/admin-pages.models';
import { euros, parseEuros } from '../prices/data-access/price-math';
import { BookingDetail, BookingsApi, Guest, STATUS_LABELS } from './data-access/bookings.api';

const GUEST_FIELDS: readonly { key: keyof Guest; label: string }[] = [
  { key: 'first_name', label: 'Prénom' },
  { key: 'last_name', label: 'Nom' },
  { key: 'email', label: 'E-mail' },
  { key: 'phone', label: 'Téléphone' },
  { key: 'street', label: 'Adresse' },
  { key: 'postcode', label: 'Code postal' },
  { key: 'town', label: 'Ville' },
  { key: 'country', label: 'Pays' },
];

const MESSAGE_STATUS: Record<string, string> = {
  pending: 'En attente',
  sending: 'En cours d’envoi',
  sent: 'Envoyé',
  failed: 'Échec',
  to_check: 'À vérifier',
  disabled: 'Non envoyé (SMS désactivés)',
};

/** Detail of a booking (F007): conditions, payments received, cancellation, history. */
@Component({
  selector: 'app-booking-detail-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [DatePipe, RouterLink, HlmButtonImports],
  template: `
    <section class="mx-auto flex max-w-4xl flex-col gap-6 px-4 py-8">
      <a routerLink="/admin/reservations" class="text-sm text-muted-foreground hover:underline">
        ← Réservations
      </a>
      @if (loadError()) {
        <p class="text-destructive" role="alert">Impossible de charger la réservation.</p>
      }
      @if (booking(); as b) {
        <div>
          <h1 class="text-2xl font-semibold">Réservation {{ b.reference }}</h1>
          <p class="mt-1" data-testid="status">
            {{ statusLabel(b) }}
            @if (b.cancel_reason) {
              — {{ b.cancel_reason }}
            }
          </p>
        </div>

        <section class="grid gap-4 rounded-lg border p-4 sm:grid-cols-2">
          <div>
            <h2 class="font-semibold">Séjour</h2>
            <p>Du {{ long(b.arrival) }} au {{ long(b.departure) }}</p>
            <p>
              {{ b.adults }} adulte(s), {{ b.children }} enfant(s) · arrivée prévue à
              {{ clock(b.arrival_time) }}
            </p>
            <p class="text-sm text-muted-foreground">
              Réservé le {{ b.created_at | date: 'dd/MM/yyyy HH:mm' }} en
              {{ b.lang === 'fr' ? 'français' : 'anglais' }} · conditions acceptées (version
              {{ b.terms.version }})
            </p>
            <p class="mt-1 text-sm">
              <a class="underline" [href]="b.tracking_link" target="_blank" rel="noopener">
                Page de suivi du voyageur
              </a>
            </p>
          </div>
          <div>
            <h2 class="font-semibold">Voyageur</h2>
            <p>{{ b.guest.first_name }} {{ b.guest.last_name }}</p>
            <p>
              <a class="underline" [href]="'mailto:' + b.guest.email">{{ b.guest.email }}</a> ·
              <a class="underline" [href]="'tel:' + b.guest.phone">{{ b.guest.phone }}</a>
            </p>
            <p>
              {{ b.guest.street }}, {{ b.guest.postcode }} {{ b.guest.town }}, {{ b.guest.country }}
            </p>
            @if (b.message) {
              <p class="mt-1 text-sm italic">« {{ b.message }} »</p>
            }
            <button
              hlmBtn
              variant="outline"
              size="sm"
              type="button"
              class="mt-2"
              (click)="editGuest(b)"
            >
              Modifier les coordonnées
            </button>
          </div>
        </section>

        @if (guestDraft(); as g) {
          <form
            class="grid gap-3 rounded-lg border p-4 sm:grid-cols-2"
            aria-label="Coordonnées du voyageur"
            (submit)="$event.preventDefault(); saveGuest(b)"
          >
            @for (field of guestFields; track field.key) {
              <label class="flex flex-col gap-1 text-sm">
                {{ field.label }}
                <input
                  #input
                  class="h-9 rounded-md border border-input px-2"
                  [name]="field.key"
                  [value]="g[field.key]"
                  (input)="patchGuest(field.key, input.value)"
                />
              </label>
            }
            <label class="flex flex-col gap-1 text-sm">
              Heure d'arrivée prévue
              <select
                #time
                class="h-9 rounded-md border border-input px-2"
                name="arrivalTime"
                (change)="arrivalTime.set(+time.value)"
              >
                @for (slot of b.arrival_slots; track slot) {
                  <option [value]="slot" [selected]="slot === arrivalTime()">
                    {{ clock(slot) }}
                  </option>
                }
              </select>
            </label>
            <div class="flex items-end gap-2">
              <button hlmBtn type="submit" [disabled]="busy()">Valider</button>
              <button hlmBtn variant="outline" type="button" (click)="guestDraft.set(null)">
                Annuler
              </button>
            </div>
          </form>
        }

        <section class="rounded-lg border p-4">
          <h2 class="font-semibold">Montants (copiés à la réservation)</h2>
          <dl class="mt-2 grid max-w-md grid-cols-[1fr_auto] gap-x-4 gap-y-1 text-sm">
            <dt>{{ b.amounts.nights }} nuit(s)</dt>
            <dd class="text-right">{{ money(b.amounts.nights_total) }}</dd>
            @if (b.amounts.promotion) {
              <dt>Promotion « {{ b.amounts.promotion }} »</dt>
              <dd class="text-right">−{{ money(b.amounts.discount) }}</dd>
            }
            <dt>Ménage</dt>
            <dd class="text-right">{{ money(b.amounts.cleaning_fee) }}</dd>
            @for (option of b.amounts.options; track option.name) {
              <dt>{{ option.name }}</dt>
              <dd class="text-right">{{ money(option.price) }}</dd>
            }
            <dt class="font-semibold">Prix du séjour</dt>
            <dd class="text-right font-semibold">{{ money(b.amounts.stay_total) }}</dd>
            <dt>
              Acompte
              @if (b.deposit_percent) {
                ({{ b.deposit_percent }} %)
              }
            </dt>
            <dd class="text-right">{{ money(b.amounts.deposit) }}</dd>
            <dt>
              Solde
              {{ b.amounts.balance_due ? 'avant le ' + short(b.amounts.balance_due) : '' }}
            </dt>
            <dd class="text-right">{{ money(b.amounts.balance) }}</dd>
            <dt>Taxe de séjour (sur place)</dt>
            <dd class="text-right">{{ money(b.amounts.tourist_tax) }}</dd>
            <dt class="font-semibold">Déjà reçu</dt>
            <dd class="text-right font-semibold" data-testid="paid">{{ money(b.amounts.paid) }}</dd>
          </dl>
        </section>

        <section class="flex flex-col gap-3 rounded-lg border p-4">
          <h2 class="font-semibold">Paiements reçus</h2>
          <ul class="text-sm">
            @for (payment of b.payments; track $index) {
              <li>
                {{ short(payment.paid_on) }} · {{ money(payment.amount) }} · {{ payment.method }}
                @if (payment.note) {
                  · {{ payment.note }}
                }
                <span class="text-muted-foreground"
                  >({{ payment.author ?? 'compte supprimé' }})</span
                >
              </li>
            } @empty {
              <li class="text-muted-foreground">Aucun paiement noté.</li>
            }
          </ul>
          @if (b.status !== 'annulee' && b.amounts.paid < b.amounts.stay_total) {
            <form
              class="flex flex-wrap items-end gap-3 text-sm"
              aria-label="Noter un paiement reçu"
              (submit)="
                $event.preventDefault();
                recordPayment(b, amount.value, paidOn.value, method.value, note.value)
              "
            >
              <label class="flex flex-col gap-1">
                Montant (€)
                <input
                  #amount
                  class="h-9 w-28 rounded-md border border-input px-2 text-right"
                  name="amount"
                  inputmode="decimal"
                  [value]="suggested(b)"
                />
              </label>
              <label class="flex flex-col gap-1">
                Reçu le
                <input
                  #paidOn
                  class="h-9 rounded-md border border-input px-2"
                  type="date"
                  name="paidOn"
                  [value]="today"
                />
              </label>
              <label class="flex flex-col gap-1">
                Mode
                <select #method class="h-9 rounded-md border border-input px-2" name="method">
                  @for (choice of methods(); track choice) {
                    <option [value]="choice">{{ choice }}</option>
                  }
                </select>
              </label>
              <label class="flex flex-col gap-1">
                Note (facultatif)
                <input #note class="h-9 rounded-md border border-input px-2" name="note" />
              </label>
              <button hlmBtn type="submit" [disabled]="busy()">Noter le paiement</button>
            </form>
          }
        </section>

        @if (errors().length > 0) {
          <ul class="list-inside list-disc text-sm text-destructive" role="alert">
            @for (error of errors(); track error) {
              <li>{{ error }}</li>
            }
          </ul>
        }

        @if (b.status !== 'annulee') {
          <section class="flex flex-col gap-2 rounded-lg border border-destructive/40 p-4">
            <h2 class="font-semibold">Annuler la réservation</h2>
            <p class="text-sm text-muted-foreground">
              Les dates sont libérées tout de suite. Le remboursement éventuel se fait hors du site.
              Le voyageur n'est pas prévenu automatiquement.
            </p>
            <label class="flex flex-col gap-1 text-sm">
              Motif
              <input
                #reason
                class="h-9 max-w-md rounded-md border border-input px-2"
                name="reason"
              />
            </label>
            <div>
              <button
                hlmBtn
                variant="destructive"
                type="button"
                [disabled]="busy()"
                (click)="cancel(b, reason.value)"
              >
                Annuler la réservation
              </button>
            </div>
          </section>
        }

        <details class="rounded-lg border p-3">
          <summary class="cursor-pointer font-semibold">Messages envoyés</summary>
          <ul class="mt-2 flex flex-col gap-1 text-sm" data-testid="messages">
            @for (message of b.messages; track message.id) {
              <li>
                {{ message.channel === 'email' ? 'E-mail' : 'SMS' }} à {{ message.recipient }} :
                {{ message.summary }} · {{ messageStatus(message.status) }}
                @if (message.error) {
                  ({{ message.error }})
                }
              </li>
            } @empty {
              <li class="text-muted-foreground">Aucun message.</li>
            }
          </ul>
        </details>

        <details class="rounded-lg border p-3">
          <summary class="cursor-pointer font-semibold">Historique</summary>
          <ul class="mt-2 flex flex-col gap-1 text-sm" data-testid="history">
            @for (event of b.events; track $index) {
              <li>
                {{ event.description }} · {{ event.author ?? 'site' }} ·
                <span class="text-muted-foreground">{{ event.at | date: 'dd/MM/yyyy HH:mm' }}</span>
              </li>
            }
          </ul>
        </details>
      }
    </section>
  `,
})
export class BookingDetailPage {
  private readonly api = inject(BookingsApi);
  private readonly paymentInfo = inject(PaymentInfoApi);

  /** Route parameter. */
  readonly id = input.required<string>();

  protected readonly guestFields = GUEST_FIELDS;
  protected readonly today = todayInParis();
  protected readonly booking = signal<BookingDetail | null>(null);
  protected readonly methods = signal<string[]>([]);
  protected readonly loadError = signal(false);
  protected readonly busy = signal(false);
  protected readonly errors = signal<string[]>([]);
  protected readonly guestDraft = signal<Guest | null>(null);
  protected readonly arrivalTime = signal(0);
  protected readonly money = euros;
  protected readonly short = formatShort;
  protected readonly clock = clockTime;

  constructor() {
    queueMicrotask(() => void this.load());
    void this.loadMethods();
  }

  protected long(date: string): string {
    return formatLong(date, { weekday: false });
  }

  protected statusLabel(booking: BookingDetail): string {
    return STATUS_LABELS[booking.status];
  }

  protected messageStatus(status: string): string {
    return MESSAGE_STATUS[status] ?? status;
  }

  /** What is still expected: the deposit, then the balance. */
  protected suggested(booking: BookingDetail): string {
    const amounts = booking.amounts;
    const left =
      amounts.paid < amounts.deposit
        ? amounts.deposit - amounts.paid
        : amounts.stay_total - amounts.paid;
    return euros(left).replace(' €', '');
  }

  protected editGuest(booking: BookingDetail): void {
    this.guestDraft.set({ ...booking.guest });
    this.arrivalTime.set(booking.arrival_time);
  }

  protected patchGuest(key: keyof Guest, value: string): void {
    this.guestDraft.update((guest) => (guest ? { ...guest, [key]: value } : guest));
  }

  protected async saveGuest(booking: BookingDetail): Promise<void> {
    const guest = this.guestDraft();
    if (
      guest &&
      (await this.act(() => this.api.updateGuest(booking.id, guest, this.arrivalTime())))
    ) {
      this.guestDraft.set(null);
    }
  }

  protected async recordPayment(
    booking: BookingDetail,
    amount: string,
    paidOn: string,
    method: string,
    note: string,
  ): Promise<void> {
    const cents = parseEuros(amount);
    if (cents === null || cents <= 0) {
      this.errors.set(['Montant : un montant en euros, par exemple 260 ou 260,50.']);
      return;
    }
    await this.act(() =>
      this.api.recordPayment(booking.id, { amount: cents, paid_on: paidOn, method, note }),
    );
  }

  protected async cancel(booking: BookingDetail, reason: string): Promise<void> {
    if (reason.trim() === '') {
      this.errors.set(["Indiquez le motif de l'annulation."]);
      return;
    }
    if (!confirm(`Annuler la réservation ${booking.reference} et libérer ses dates ?`)) {
      return;
    }
    await this.act(() => this.api.cancel(booking.id, reason));
  }

  /** Runs a change, then reloads; false if it was refused. */
  private async act(change: () => Observable<unknown>): Promise<boolean> {
    this.busy.set(true);
    this.errors.set([]);
    try {
      await firstValueFrom(change());
      await this.load();
      return true;
    } catch (error) {
      this.errors.set(messages(error));
      return false;
    } finally {
      this.busy.set(false);
    }
  }

  private async load(): Promise<void> {
    try {
      this.booking.set(await firstValueFrom(this.api.detail(Number(this.id()))));
      this.loadError.set(false);
    } catch {
      this.loadError.set(true);
    }
  }

  private async loadMethods(): Promise<void> {
    try {
      this.methods.set((await firstValueFrom(this.paymentInfo.info())).methods);
    } catch {
      this.methods.set([]);
    }
  }
}

function messages(error: unknown): string[] {
  const body = error instanceof HttpErrorResponse ? (error.error as ApiErrorBody | null) : null;
  const details = body?.error?.details ?? [];
  if (details.length > 0) {
    return details;
  }
  return [body?.error?.message ?? 'Une erreur est survenue, réessayez plus tard.'];
}
