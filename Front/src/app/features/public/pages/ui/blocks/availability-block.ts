import { HttpErrorResponse } from '@angular/common/http';
import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  effect,
  inject,
  input,
  signal,
  untracked,
} from '@angular/core';
import { Router } from '@angular/router';
import { firstValueFrom } from 'rxjs';

import {
  IsoDate,
  addDays,
  monthGrid,
  nightsBetween,
  shiftMonth,
  todayInParis,
} from '../../../../../shared/dates/dates';
import { AvailabilityApi, PublicCalendar, Quote } from '../../data-access/availability.api';
import { AvailabilityBlock, Lang } from '../../data-access/pages.models';

const TEXT = {
  fr: {
    previous: 'Mois précédent',
    next: 'Mois suivant',
    weekdays: ['lun.', 'mar.', 'mer.', 'jeu.', 'ven.', 'sam.', 'dim.'],
    arrival: 'Arrivée',
    departure: 'Départ',
    adults: 'Adultes',
    children: (age: number) => `Enfants (moins de ${age} ans)`,
    free: 'libre',
    unavailable: 'indisponible',
    pick: 'Choisissez votre date d’arrivée puis votre date de départ.',
    pickDeparture: 'Choisissez maintenant votre date de départ.',
    nights: (n: number) => (n > 1 ? `${n} nuits` : '1 nuit'),
    cleaning: 'Ménage',
    total: 'Total du séjour',
    deposit: 'Acompte à la réservation',
    balance: (date: string) => `Solde à régler avant le ${date}`,
    allAtBooking: 'La totalité est due à la réservation.',
    tax: 'Taxe de séjour, à régler sur place',
    minimum: (n: number) => `${n} nuits minimum`,
    refused: 'Ces dates ne sont pas réservables :',
    error: 'Le devis n’a pas pu être calculé, réessayez plus tard.',
    loadError: 'Le calendrier n’a pas pu être chargé.',
    calendar: 'Disponibilités et prix par nuit',
    perNight: 'par nuit',
  },
  en: {
    previous: 'Previous month',
    next: 'Next month',
    weekdays: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'],
    arrival: 'Arrival',
    departure: 'Departure',
    adults: 'Adults',
    children: (age: number) => `Children (under ${age})`,
    free: 'available',
    unavailable: 'unavailable',
    pick: 'Choose your arrival date, then your departure date.',
    pickDeparture: 'Now choose your departure date.',
    nights: (n: number) => (n > 1 ? `${n} nights` : '1 night'),
    cleaning: 'Cleaning',
    total: 'Stay total',
    deposit: 'Deposit at booking',
    balance: (date: string) => `Balance due by ${date}`,
    allAtBooking: 'The full amount is due at booking.',
    tax: 'Tourist tax, paid on arrival',
    minimum: (n: number) => `${n} nights minimum`,
    refused: 'These dates cannot be booked:',
    error: 'The quote could not be computed, please try again later.',
    loadError: 'The calendar could not be loaded.',
    calendar: 'Availability and nightly prices',
    perNight: 'per night',
  },
};

interface Month {
  year: number;
  month: number;
}

function monthOf(day: IsoDate): Month {
  return { year: Number(day.slice(0, 4)), month: Number(day.slice(5, 7)) - 1 };
}

function firstOf({ year, month }: Month): IsoDate {
  return `${year}-${String(month + 1).padStart(2, '0')}-01`;
}

/**
 * "Disponibilités" block (F004): months with the site price under each free night, arrival and
 * departure chosen in the calendar or typed, and the exact quote computed by the API.
 */
@Component({
  selector: 'app-availability-block',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @let t = text();
    <section class="mx-auto flex max-w-5xl flex-col gap-4 px-4" [attr.aria-label]="t.calendar">
      <div class="flex items-center justify-between">
        <button
          type="button"
          class="rounded-md border px-3 py-1 disabled:opacity-40"
          [attr.aria-label]="t.previous"
          [disabled]="!canGoBack()"
          (click)="move(-1)"
        >
          ‹
        </button>
        <button
          type="button"
          class="rounded-md border px-3 py-1 disabled:opacity-40"
          [attr.aria-label]="t.next"
          [disabled]="!canGoForward()"
          (click)="move(1)"
        >
          ›
        </button>
      </div>

      @if (loadError()) {
        <p class="text-sm text-destructive" role="alert">{{ t.loadError }}</p>
      }

      <div
        class="grid gap-6"
        role="group"
        tabindex="-1"
        [attr.aria-label]="t.calendar"
        [class]="gridClass()"
        (keydown)="onKey($event)"
        (touchstart)="touchStart($event)"
        (touchend)="touchEnd($event)"
      >
        @for (shown of months(); track $index; let index = $index) {
          <div [class]="index > 0 ? '@max-3xl:hidden' : ''" data-testid="month">
            <h3 class="mb-2 text-center font-semibold capitalize">{{ monthTitle(shown) }}</h3>
            <div
              class="grid grid-cols-7 gap-1 text-center text-xs text-muted-foreground"
              aria-hidden="true"
            >
              @for (day of t.weekdays; track day) {
                <span>{{ day }}</span>
              }
            </div>
            <div class="mt-1 grid grid-cols-7 gap-1">
              @for (day of grid(shown); track $index) {
                @if (day) {
                  @let night = nightOf(day);
                  <button
                    type="button"
                    class="flex aspect-square flex-col items-center justify-center rounded-md border text-xs"
                    [class.bg-muted]="!night?.price"
                    [class.text-muted-foreground]="!night?.price"
                    [class.line-through]="!night?.price && !isDeparture(day)"
                    [class.bg-primary]="inStay(day)"
                    [class.text-primary-foreground]="inStay(day)"
                    [class.border-primary]="isDeparture(day)"
                    [class.border-2]="isDeparture(day)"
                    [attr.data-date]="day"
                    [attr.aria-label]="dayLabel(day)"
                    [attr.aria-pressed]="inStay(day) || isDeparture(day)"
                    [attr.aria-disabled]="!selectable(day)"
                    [tabindex]="day === focused() ? 0 : -1"
                    (click)="select(day)"
                    (focus)="focused.set(day)"
                  >
                    <span class="font-medium">{{ +day.slice(8) }}</span>
                    @if (night?.price; as price) {
                      <span data-testid="price">{{ money(price) }}</span>
                    }
                  </button>
                } @else {
                  <span></span>
                }
              }
            </div>
          </div>
        }
      </div>

      <form class="flex flex-wrap items-end gap-3 text-sm" (submit)="$event.preventDefault()">
        <label class="flex flex-col gap-1">
          {{ t.arrival }}
          <input
            #arrivalField
            class="h-9 rounded-md border border-input bg-background px-2"
            type="date"
            name="arrival"
            [min]="calendar()?.earliest_arrival ?? today"
            [value]="arrival() ?? ''"
            (change)="typeArrival(arrivalField.value)"
          />
        </label>
        <label class="flex flex-col gap-1">
          {{ t.departure }}
          <input
            #departureField
            class="h-9 rounded-md border border-input bg-background px-2"
            type="date"
            name="departure"
            [min]="arrival() ?? today"
            [value]="departure() ?? ''"
            (change)="typeDeparture(departureField.value)"
          />
        </label>
        <label class="flex flex-col gap-1">
          {{ t.adults }}
          <input
            #adultsField
            class="h-9 w-20 rounded-md border border-input bg-background px-2 text-right"
            type="number"
            name="adults"
            min="1"
            [max]="calendar()?.capacity ?? null"
            [value]="adults()"
            (input)="adults.set(+adultsField.value || 0)"
          />
        </label>
        <label class="flex flex-col gap-1">
          {{ t.children(calendar()?.children_under ?? 18) }}
          <input
            #childrenField
            class="h-9 w-20 rounded-md border border-input bg-background px-2 text-right"
            type="number"
            name="children"
            min="0"
            [value]="children()"
            (input)="children.set(+childrenField.value || 0)"
          />
        </label>
      </form>

      <div aria-live="polite" data-testid="quote">
        @if (refusals().length > 0) {
          <div class="rounded-md bg-muted p-3 text-sm" role="alert">
            <p>{{ t.refused }}</p>
            <ul class="list-inside list-disc">
              @for (reason of refusals(); track reason) {
                <li>{{ reason }}</li>
              }
            </ul>
          </div>
        } @else if (quote(); as q) {
          <dl
            class="grid max-w-md grid-cols-[1fr_auto] gap-x-6 gap-y-1 rounded-md border p-4 text-sm"
          >
            <dt>{{ t.nights(q.nights) }}</dt>
            <dd class="text-right">{{ money(q.nights_total) }}</dd>
            <dt>{{ t.cleaning }}</dt>
            <dd class="text-right">{{ money(q.cleaning_fee) }}</dd>
            <dt class="font-semibold">{{ t.total }}</dt>
            <dd class="text-right font-semibold" data-testid="stay-total">
              {{ money(q.stay_total) }}
            </dd>
            <dt>{{ t.deposit }}</dt>
            <dd class="text-right">{{ money(q.deposit) }}</dd>
            @if (q.balance_due) {
              <dt>{{ t.balance(shortDate(q.balance_due)) }}</dt>
              <dd class="text-right">{{ money(q.balance) }}</dd>
            } @else {
              <dt class="col-span-2 text-muted-foreground">{{ t.allAtBooking }}</dt>
            }
            @if (q.tourist_tax > 0) {
              <dt class="text-muted-foreground">{{ t.tax }}</dt>
              <dd class="text-right text-muted-foreground" data-testid="tourist-tax">
                {{ money(q.tourist_tax) }}
              </dd>
            }
          </dl>
        } @else if (quoteError()) {
          <p class="text-sm text-destructive" role="alert">{{ t.error }}</p>
        } @else {
          <p class="text-sm text-muted-foreground">
            {{ arrival() && !departure() ? t.pickDeparture : t.pick }}
          </p>
        }
      </div>
    </section>
  `,
})
export class AvailabilityBlockView {
  readonly block = input.required<AvailabilityBlock>();

  private readonly api = inject(AvailabilityApi);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly lang: Lang = inject(Router).url.startsWith('/en') ? 'en' : 'fr';
  protected readonly text = signal(TEXT[this.lang]);

  protected readonly today = todayInParis();
  private readonly start = signal<Month>(monthOf(this.today));
  protected readonly months = computed(() =>
    Array.from({ length: this.block().months }, (_, index) =>
      shiftMonth(this.start().year, this.start().month, index),
    ),
  );
  protected readonly gridClass = computed(
    () => ({ 1: '', 2: '@3xl:grid-cols-2', 3: '@3xl:grid-cols-3' })[this.block().months] ?? '',
  );

  protected readonly calendar = signal<PublicCalendar | null>(null);
  protected readonly loadError = signal(false);
  private readonly nights = computed(
    () => new Map((this.calendar()?.nights ?? []).map((night) => [night.date, night])),
  );

  protected readonly arrival = signal<IsoDate | null>(null);
  protected readonly departure = signal<IsoDate | null>(null);
  protected readonly adults = signal(2);
  protected readonly children = signal(0);
  protected readonly quote = signal<Quote | null>(null);
  protected readonly refusals = signal<string[]>([]);
  protected readonly quoteError = signal(false);
  protected readonly focused = signal<IsoDate>(this.today);

  protected readonly canGoBack = computed(
    () => firstOf(this.start()) > firstOf(monthOf(this.today)),
  );
  protected readonly canGoForward = computed(() => {
    const lastNight = this.calendar()?.last_night;
    const months = this.months();
    return (
      !lastNight || firstOf(shiftMonth(months.at(-1)!.year, months.at(-1)!.month, 1)) <= lastNight
    );
  });

  private touchX: number | null = null;

  constructor() {
    effect(() => {
      const months = this.months();
      const from = firstOf(months[0]);
      const to = firstOf(shiftMonth(months.at(-1)!.year, months.at(-1)!.month, 1));
      void untracked(() => this.load(from, to));
    });
    effect(() => {
      const arrival = this.arrival();
      const departure = this.departure();
      const adults = this.adults();
      const children = this.children();
      untracked(() => {
        if (arrival && departure) {
          void this.ask(arrival, departure, adults, children);
        } else {
          this.quote.set(null);
          this.refusals.set([]);
          this.quoteError.set(false);
        }
      });
    });
  }

  protected grid(month: Month): (IsoDate | null)[] {
    return monthGrid(month.year, month.month);
  }

  protected monthTitle(month: Month): string {
    return new Intl.DateTimeFormat(this.lang === 'en' ? 'en-GB' : 'fr-FR', {
      timeZone: 'UTC',
      month: 'long',
      year: 'numeric',
    }).format(new Date(Date.UTC(month.year, month.month, 1)));
  }

  protected nightOf(day: IsoDate) {
    return this.nights().get(day) ?? null;
  }

  protected money(cents: number): string {
    return new Intl.NumberFormat(this.lang === 'en' ? 'en-GB' : 'fr-FR', {
      style: 'currency',
      currency: 'EUR',
      minimumFractionDigits: cents % 100 === 0 ? 0 : 2,
    }).format(cents / 100);
  }

  protected shortDate(day: IsoDate): string {
    const [year, month, date] = day.split('-');
    return `${date}/${month}/${year}`;
  }

  protected inStay(day: IsoDate): boolean {
    const arrival = this.arrival();
    const departure = this.departure();
    return arrival !== null && (departure ? day >= arrival && day < departure : day === arrival);
  }

  protected isDeparture(day: IsoDate): boolean {
    return day === this.departure();
  }

  /** A free night can start a stay; once the arrival is chosen, any later day can end it. */
  protected selectable(day: IsoDate): boolean {
    const arrival = this.arrival();
    const choosingDeparture = arrival !== null && this.departure() === null && day > arrival;
    return choosingDeparture || this.nightOf(day)?.price != null;
  }

  protected dayLabel(day: IsoDate): string {
    const t = this.text();
    const night = this.nightOf(day);
    const date = new Intl.DateTimeFormat(this.lang === 'en' ? 'en-GB' : 'fr-FR', {
      timeZone: 'UTC',
      weekday: 'long',
      day: 'numeric',
      month: 'long',
    }).format(new Date(`${day}T00:00:00Z`));
    if (!night?.price) {
      return `${date}, ${t.unavailable}`;
    }
    const minimum =
      night.min_nights && night.min_nights > 1 ? `, ${t.minimum(night.min_nights)}` : '';
    return `${date}, ${t.free}, ${this.money(night.price)} ${t.perNight}${minimum}`;
  }

  protected select(day: IsoDate): void {
    if (!this.selectable(day)) {
      return;
    }
    const arrival = this.arrival();
    if (arrival && !this.departure() && day > arrival) {
      this.departure.set(day);
    } else {
      this.arrival.set(day);
      this.departure.set(null);
    }
  }

  protected typeArrival(value: string): void {
    this.arrival.set(value || null);
    const departure = this.departure();
    if (value && departure && departure <= value) {
      this.departure.set(null);
    }
    if (value) {
      this.show(value);
    }
  }

  protected typeDeparture(value: string): void {
    this.departure.set(value || null);
  }

  protected move(delta: number): void {
    if ((delta < 0 && !this.canGoBack()) || (delta > 0 && !this.canGoForward())) {
      return;
    }
    this.start.update(({ year, month }) => shiftMonth(year, month, delta));
  }

  /** Arrow keys move between days (changing month when needed). */
  protected onKey(event: KeyboardEvent): void {
    const steps: Record<string, number> = {
      ArrowLeft: -1,
      ArrowRight: 1,
      ArrowUp: -7,
      ArrowDown: 7,
    };
    const step = steps[event.key];
    if (step === undefined) {
      return;
    }
    event.preventDefault();
    const next = addDays(this.focused(), step);
    if (next < this.today) {
      return;
    }
    this.focused.set(next);
    this.show(next);
    setTimeout(() =>
      this.host.nativeElement.querySelector<HTMLButtonElement>(`[data-date="${next}"]`)?.focus(),
    );
  }

  protected touchStart(event: TouchEvent): void {
    this.touchX = event.changedTouches[0]?.clientX ?? null;
  }

  protected touchEnd(event: TouchEvent): void {
    const end = event.changedTouches[0]?.clientX;
    if (this.touchX === null || end === undefined) {
      return;
    }
    const distance = end - this.touchX;
    this.touchX = null;
    if (Math.abs(distance) > 50) {
      this.move(distance < 0 ? 1 : -1);
    }
  }

  /** Brings the month of `day` into view. */
  private show(day: IsoDate): void {
    const months = this.months();
    const first = firstOf(months[0]);
    const after = firstOf(shiftMonth(months.at(-1)!.year, months.at(-1)!.month, 1));
    if (day < first || day >= after) {
      this.start.set(monthOf(day));
    }
  }

  private async load(from: IsoDate, to: IsoDate): Promise<void> {
    try {
      this.calendar.set(await firstValueFrom(this.api.calendar(from, to)));
      this.loadError.set(false);
    } catch {
      this.loadError.set(true);
    }
  }

  private async ask(
    arrival: IsoDate,
    departure: IsoDate,
    adults: number,
    children: number,
  ): Promise<void> {
    if (departure <= arrival || nightsBetween(arrival, departure) < 1) {
      return;
    }
    try {
      const quote = await firstValueFrom(this.api.quote({ arrival, departure, adults, children }));
      if (this.arrival() === arrival && this.departure() === departure) {
        this.quote.set(quote);
        this.refusals.set([]);
        this.quoteError.set(false);
      }
    } catch (error) {
      const details =
        error instanceof HttpErrorResponse && error.status === 422
          ? ((error.error as { error?: { details?: string[] } } | null)?.error?.details ?? [])
          : [];
      this.quote.set(null);
      this.refusals.set(details);
      this.quoteError.set(details.length === 0);
    }
  }
}
