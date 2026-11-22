import { HttpErrorResponse } from '@angular/common/http';
import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  input,
  signal,
  untracked,
} from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';
import { firstValueFrom } from 'rxjs';

import { CurrentPage } from '../pages/data-access/current-page';
import { Lang } from '../pages/data-access/pages.models';
import { BOOKING_TEXT, clock, longDate, money } from './booking-texts';
import { BookingApi, TrackedBooking } from './data-access/booking.api';

/**
 * The guest's tracking page (F007), reached by the random link of the confirmation email: the
 * stay, amounts, payments received, how to pay, and the expected arrival time.
 */
@Component({
  selector: 'app-tracking-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @let t = text();
    <section class="mx-auto flex max-w-3xl flex-col gap-6 px-4 py-10">
      <h1 class="text-3xl font-semibold tracking-tight">{{ t.tracking }}</h1>
      @if (booking(); as b) {
        @if (confirmed()) {
          <p class="rounded-md bg-primary/10 p-3" role="status" data-testid="confirmed">
            {{ t.confirmed(b.reference) }}
          </p>
        }
        <section class="rounded-lg border p-4">
          <p>
            <span class="font-medium">{{ t.reference }} :</span> {{ b.reference }} ·
            <span data-testid="status">{{ t.status[b.status] }}</span>
          </p>
          <p>{{ t.from(long(b.arrival), long(b.departure)) }}</p>
          <p>{{ t.guests(b.adults, b.children) }}</p>
          <dl class="mt-3 grid max-w-md grid-cols-[1fr_auto] gap-x-4 gap-y-1 text-sm">
            <dt>{{ t.nights(b.amounts.nights) }}</dt>
            <dd class="text-right">{{ eur(b.amounts.nights_total) }}</dd>
            @if (b.amounts.promotion) {
              <dt>{{ t.promotion(b.amounts.promotion) }}</dt>
              <dd class="text-right">−{{ eur(b.amounts.discount) }}</dd>
            }
            @if (b.amounts.cleaning_fee > 0) {
              <dt>{{ t.cleaning }}</dt>
              <dd class="text-right">{{ eur(b.amounts.cleaning_fee) }}</dd>
            }
            @for (option of b.amounts.options; track option.name) {
              <dt>{{ option.name }}</dt>
              <dd class="text-right">{{ eur(option.price) }}</dd>
            }
            <dt class="font-semibold">{{ t.total }}</dt>
            <dd class="text-right font-semibold">{{ eur(b.amounts.stay_total) }}</dd>
            <dt>{{ t.deposit }}</dt>
            <dd class="text-right">{{ eur(b.amounts.deposit) }}</dd>
            @if (b.amounts.balance_due) {
              <dt>{{ t.balance(long(b.amounts.balance_due)) }}</dt>
              <dd class="text-right">{{ eur(b.amounts.balance) }}</dd>
            }
            <dt>{{ t.paid }}</dt>
            <dd class="text-right" data-testid="paid">{{ eur(b.amounts.paid) }}</dd>
            @if (b.amounts.tourist_tax > 0) {
              <dt class="text-muted-foreground">{{ t.tax }}</dt>
              <dd class="text-right text-muted-foreground">{{ eur(b.amounts.tourist_tax) }}</dd>
            }
          </dl>
        </section>

        <section class="rounded-lg border p-4">
          <h2 class="text-lg font-semibold">{{ t.rules }}</h2>
          <ul class="mt-1 list-inside list-disc text-sm">
            <li>{{ t.arrivalFrom(time(b.arrival_from)) }}</li>
            <li>{{ t.departureBefore(time(b.departure_before)) }}</li>
            <li>{{ t.pets(b.pets_allowed) }}</li>
          </ul>
          @if (b.status !== 'annulee' && b.amounts.paid < b.amounts.stay_total && b.instructions) {
            <h3 class="mt-3 font-semibold">{{ t.howToPay }}</h3>
            <div class="rich-text text-sm" [innerHTML]="b.instructions"></div>
          }
        </section>

        @if (b.status !== 'annulee') {
          <section class="flex flex-col gap-2 rounded-lg border p-4">
            <h2 class="text-lg font-semibold">{{ t.arrivalTime }} : {{ time(b.arrival_time) }}</h2>
            @if (b.arrival_time_changeable) {
              <form
                class="flex flex-wrap items-end gap-2 text-sm"
                (submit)="$event.preventDefault(); changeTime(+slot.value)"
              >
                <label class="flex flex-col gap-1">
                  {{ t.changeTime }}
                  <select
                    #slot
                    class="h-10 w-32 rounded-md border border-input px-2"
                    name="arrivalTime"
                  >
                    @for (minutes of b.arrival_slots; track minutes) {
                      <option [value]="minutes" [selected]="minutes === b.arrival_time">
                        {{ time(minutes) }}
                      </option>
                    }
                  </select>
                </label>
                <button
                  type="submit"
                  class="inline-flex h-10 items-center rounded-md bg-primary px-4 font-medium text-primary-foreground hover:opacity-90"
                >
                  {{ t.save }}
                </button>
              </form>
              @if (timeSaved()) {
                <p class="text-sm" role="status">{{ t.timeSaved }}</p>
              }
            } @else {
              <p class="text-sm text-muted-foreground">{{ t.tooLate }}</p>
            }
            @if (timeError()) {
              <p class="text-sm text-destructive" role="alert">{{ timeError() }}</p>
            }
          </section>
        }
      } @else if (notFound()) {
        <p role="alert">{{ t.notFound }}</p>
      } @else if (loadError()) {
        <p role="alert">{{ t.error }}</p>
      }
    </section>
  `,
})
export class TrackingPage {
  readonly lang = input<Lang>('fr');
  readonly token = input.required<string>();
  /** Set when arriving from the booking form. */
  readonly confirmed = input<string | undefined>();

  private readonly api = inject(BookingApi);
  private readonly current = inject(CurrentPage);
  private readonly title = inject(Title);
  private readonly meta = inject(Meta);

  protected readonly booking = signal<TrackedBooking | null>(null);
  protected readonly notFound = signal(false);
  protected readonly loadError = signal(false);
  protected readonly timeSaved = signal(false);
  protected readonly timeError = signal('');
  /** The booking's own language once loaded, the page's before. */
  private readonly shown = computed(() => this.booking()?.lang ?? this.lang());
  protected readonly text = computed(() => BOOKING_TEXT[this.shown()]);

  constructor() {
    effect(() => {
      const token = this.token();
      this.current.alternates.set({
        fr: `/reservation/${token}`,
        en: `/en/booking/${token}`,
      });
      this.meta.updateTag({ name: 'robots', content: 'noindex' });
      this.title.setTitle(`${BOOKING_TEXT[this.lang()].tracking} - La Petite Grange`);
      void untracked(() => this.load(token));
    });
  }

  protected eur(cents: number): string {
    return money(cents, this.shown());
  }

  protected long(date: string): string {
    return longDate(date, this.shown());
  }

  protected time(minutes: number): string {
    return clock(minutes);
  }

  protected async changeTime(minutes: number): Promise<void> {
    this.timeSaved.set(false);
    this.timeError.set('');
    try {
      await firstValueFrom(this.api.changeArrivalTime(this.token(), minutes));
      await this.load(this.token());
      this.timeSaved.set(true);
    } catch (error) {
      const body =
        error instanceof HttpErrorResponse
          ? (error.error as { error?: { details?: string[]; message?: string } } | null)
          : null;
      this.timeError.set(
        body?.error?.details?.join(' ') ?? body?.error?.message ?? this.text().error,
      );
    }
  }

  private async load(token: string): Promise<void> {
    try {
      this.booking.set(await firstValueFrom(this.api.track(token)));
      this.notFound.set(false);
      this.loadError.set(false);
    } catch (error) {
      this.booking.set(null);
      if (error instanceof HttpErrorResponse && error.status === 404) {
        this.notFound.set(true);
      } else {
        this.loadError.set(true);
      }
    }
  }
}
