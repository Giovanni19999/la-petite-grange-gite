import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { firstValueFrom } from 'rxjs';

import { formatShort } from '../../../shared/dates/dates';
import { euros } from '../prices/data-access/price-math';
import {
  BookingFilter,
  BookingStatus,
  BookingSummary,
  BookingsApi,
  STATUS_LABELS,
} from './data-access/bookings.api';

/** Payment state of a booking, as the list shows it. */
export function paymentState(booking: BookingSummary): string {
  if (booking.status === 'annulee') {
    return '—';
  }
  if (booking.paid >= booking.stay_total) {
    return 'Tout est payé';
  }
  if (booking.paid < booking.deposit) {
    return booking.deposit_late ? 'Acompte en retard' : 'Acompte attendu';
  }
  return booking.balance_late ? 'Solde en retard' : 'Solde attendu';
}

/** "Réservations" (F007): direct bookings, stays to come first, with their payments. */
@Component({
  selector: 'app-bookings-list-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink],
  template: `
    <section class="mx-auto flex max-w-5xl flex-col gap-6 px-4 py-8">
      <div>
        <a routerLink="/admin" class="text-sm text-muted-foreground hover:underline">
          ← Espace administrateur
        </a>
        <h1 class="text-2xl font-semibold">Réservations</h1>
        <p class="mt-1 text-sm text-muted-foreground">
          Réservations faites sur le site, confirmées tout de suite. Notez les paiements reçus dans
          le détail de chaque réservation.
        </p>
      </div>

      <form
        class="flex flex-wrap items-end gap-3 text-sm"
        (submit)="$event.preventDefault(); load()"
        aria-label="Filtres"
      >
        <label class="flex flex-col gap-1">
          Statut
          <select
            #status
            class="h-9 rounded-md border border-input px-2"
            name="status"
            (change)="filter.status = $any(status.value); load()"
          >
            <option value="">Tous</option>
            @for (entry of statuses; track entry.value) {
              <option [value]="entry.value">{{ entry.label }}</option>
            }
          </select>
        </label>
        <label class="flex flex-col gap-1">
          Séjours après le
          <input
            #from
            class="h-9 rounded-md border border-input px-2"
            type="date"
            name="from"
            (change)="filter.from = from.value; load()"
          />
        </label>
        <label class="flex flex-col gap-1">
          et avant le
          <input
            #to
            class="h-9 rounded-md border border-input px-2"
            type="date"
            name="to"
            (change)="filter.to = to.value; load()"
          />
        </label>
      </form>

      @if (error()) {
        <p class="text-destructive" role="alert">Impossible de charger les réservations.</p>
      }

      <div class="overflow-x-auto">
        <table class="w-full text-sm" data-testid="bookings">
          <thead>
            <tr class="border-b text-left">
              <th class="py-2 pr-3">Référence</th>
              <th class="py-2 pr-3">Voyageur</th>
              <th class="py-2 pr-3">Séjour</th>
              <th class="py-2 pr-3 text-right">Prix</th>
              <th class="py-2 pr-3">Statut</th>
              <th class="py-2">Paiement</th>
            </tr>
          </thead>
          <tbody>
            @for (booking of bookings(); track booking.id) {
              <tr class="border-b" [attr.data-booking]="booking.id">
                <td class="py-2 pr-3">
                  <a class="underline" [routerLink]="['/admin/reservations', booking.id]">
                    {{ booking.reference }}
                  </a>
                </td>
                <td class="py-2 pr-3">{{ booking.guest }} ({{ booking.guests }})</td>
                <td class="py-2 pr-3">
                  {{ short(booking.arrival) }} → {{ short(booking.departure) }}
                </td>
                <td class="py-2 pr-3 text-right">{{ money(booking.stay_total) }}</td>
                <td class="py-2 pr-3">{{ label(booking.status) }}</td>
                <td
                  class="py-2"
                  [class.text-destructive]="booking.deposit_late || booking.balance_late"
                >
                  {{ payment(booking) }}
                </td>
              </tr>
            } @empty {
              <tr>
                <td colspan="6" class="py-4 text-muted-foreground">Aucune réservation.</td>
              </tr>
            }
          </tbody>
        </table>
      </div>
    </section>
  `,
})
export class BookingsListPage {
  private readonly api = inject(BookingsApi);

  protected readonly statuses = (Object.keys(STATUS_LABELS) as BookingStatus[]).map((value) => ({
    value,
    label: STATUS_LABELS[value],
  }));
  protected readonly filter: BookingFilter = {};
  protected readonly bookings = signal<BookingSummary[]>([]);
  protected readonly error = signal(false);
  protected readonly short = formatShort;
  protected readonly money = euros;
  protected readonly payment = paymentState;

  constructor() {
    void this.load();
  }

  protected label(status: BookingStatus): string {
    return STATUS_LABELS[status];
  }

  protected async load(): Promise<void> {
    try {
      this.bookings.set(await firstValueFrom(this.api.list(this.filter)));
      this.error.set(false);
    } catch {
      this.error.set(true);
    }
  }
}
