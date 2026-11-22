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
import { Router, RouterLink } from '@angular/router';
import { firstValueFrom } from 'rxjs';

import { AvailabilityApi, Quote } from '../pages/data-access/availability.api';
import { CurrentPage } from '../pages/data-access/current-page';
import { Lang } from '../pages/data-access/pages.models';
import { BOOKING_TEXT, clock, longDate, money } from './booking-texts';
import { BookingApi, BookingInfo, GuestDetails } from './data-access/booking.api';

interface ApiError {
  error?: { code?: string; message?: string; details?: string[] };
}

const GUEST_FIELDS: readonly {
  key: keyof GuestDetails;
  type: string;
  autocomplete: string;
}[] = [
  { key: 'first_name', type: 'text', autocomplete: 'given-name' },
  { key: 'last_name', type: 'text', autocomplete: 'family-name' },
  { key: 'email', type: 'email', autocomplete: 'email' },
  { key: 'phone', type: 'tel', autocomplete: 'tel' },
  { key: 'street', type: 'text', autocomplete: 'street-address' },
  { key: 'postcode', type: 'text', autocomplete: 'postal-code' },
  { key: 'town', type: 'text', autocomplete: 'address-level2' },
  { key: 'country', type: 'text', autocomplete: 'country-name' },
];

const LABELS: Record<
  keyof GuestDetails,
  'firstName' | 'lastName' | 'email' | 'phone' | 'street' | 'postcode' | 'town' | 'country'
> = {
  first_name: 'firstName',
  last_name: 'lastName',
  email: 'email',
  phone: 'phone',
  street: 'street',
  postcode: 'postcode',
  town: 'town',
  country: 'country',
};

/**
 * Direct booking page (F007, ADR-0020): the quote recomputed by the API, the house rules and how
 * to pay, the guest's details and the rental terms. Confirmed at once, paid offline.
 */
@Component({
  selector: 'app-booking-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink],
  template: `
    @let t = text();
    <section class="mx-auto flex max-w-3xl flex-col gap-6 px-4 py-10">
      <h1 class="text-3xl font-semibold tracking-tight">{{ t.title }}</h1>

      @if (!arrival() || !departure()) {
        <p>
          {{ t.missingStay }} <a class="underline" [routerLink]="home()">{{ t.back }}</a>
        </p>
      } @else if (info(); as i) {
        @if (!i.open) {
          <p role="alert">{{ t.closed }}</p>
        } @else {
          <section class="rounded-lg border p-4" aria-labelledby="stay-title">
            <h2 id="stay-title" class="text-lg font-semibold">{{ t.stay }}</h2>
            <p>{{ t.from(long(arrival()!), long(departure()!)) }}</p>
            <p>{{ t.guests(adultCount(), childCount()) }}</p>
            @if (quote(); as q) {
              <dl
                class="mt-3 grid max-w-md grid-cols-[1fr_auto] gap-x-4 gap-y-1 text-sm"
                data-testid="amounts"
              >
                <dt>{{ t.nights(q.nights) }}</dt>
                <dd class="text-right">{{ eur(q.nights_total) }}</dd>
                @if (q.promotion; as promotion) {
                  <dt>{{ t.promotion(promotion.name) }}</dt>
                  <dd class="text-right">−{{ eur(promotion.discount) }}</dd>
                }
                @if (q.cleaning_fee > 0) {
                  <dt>{{ t.cleaning }}</dt>
                  <dd class="text-right">{{ eur(q.cleaning_fee) }}</dd>
                }
                @for (option of chosenOptions(); track option.id) {
                  <dt>{{ option.name }}</dt>
                  <dd class="text-right">{{ eur(option.price) }}</dd>
                }
                <dt class="font-semibold">{{ t.total }}</dt>
                <dd class="text-right font-semibold">{{ eur(q.stay_total) }}</dd>
                <dt>{{ t.deposit }}</dt>
                <dd class="text-right">{{ eur(q.deposit) }}</dd>
                @if (q.balance_due) {
                  <dt>{{ t.balance(long(q.balance_due)) }}</dt>
                  <dd class="text-right">{{ eur(q.balance) }}</dd>
                } @else {
                  <dt class="col-span-2 text-muted-foreground">{{ t.allAtBooking }}</dt>
                }
                @if (q.tourist_tax > 0) {
                  <dt class="text-muted-foreground">{{ t.tax }}</dt>
                  <dd class="text-right text-muted-foreground">{{ eur(q.tourist_tax) }}</dd>
                }
              </dl>
            } @else if (quoteErrors().length > 0) {
              <div role="alert" class="mt-2 text-sm text-destructive">
                <p>{{ t.refused }}</p>
                <ul class="list-inside list-disc">
                  @for (error of quoteErrors(); track error) {
                    <li>{{ error }}</li>
                  }
                </ul>
                <a class="underline" [routerLink]="home()">{{ t.back }}</a>
              </div>
            }
          </section>

          <section class="rounded-lg border p-4" aria-labelledby="rules-title" data-testid="rules">
            <h2 id="rules-title" class="text-lg font-semibold">{{ t.rules }}</h2>
            <ul class="mt-1 list-inside list-disc text-sm">
              <li>{{ t.arrivalFrom(time(i.arrival_from)) }}</li>
              <li>{{ t.departureBefore(time(i.departure_before)) }}</li>
              <li>{{ t.capacity(i.capacity) }}</li>
              <li>{{ t.pets(i.pets_allowed) }}</li>
            </ul>
            <h3 class="mt-3 font-semibold">{{ t.howToPay }}</h3>
            <p class="text-sm">{{ t.noOnlinePayment }}</p>
            @if (i.instructions) {
              <div class="rich-text mt-2 text-sm" [innerHTML]="i.instructions"></div>
            }
            @if (i.document) {
              <p class="mt-2 text-sm">{{ t.document }}</p>
            }
          </section>

          @if (quote()) {
            <form
              class="flex flex-col gap-4 rounded-lg border p-4"
              aria-labelledby="details-title"
              (submit)="$event.preventDefault(); submit()"
            >
              <h2 id="details-title" class="text-lg font-semibold">{{ t.details }}</h2>
              <div class="grid gap-3 sm:grid-cols-2">
                @for (field of guestFields; track field.key) {
                  <label class="flex flex-col gap-1 text-sm">
                    {{ t[labels[field.key]] }}
                    <input
                      #input
                      class="h-10 rounded-md border border-input px-2"
                      required
                      [name]="field.key"
                      [type]="field.type"
                      [attr.autocomplete]="field.autocomplete"
                      [value]="guest()[field.key]"
                      (input)="patchGuest(field.key, input.value)"
                    />
                  </label>
                }
              </div>
              <label class="flex flex-col gap-1 text-sm">
                {{ t.arrivalTime }}
                <select
                  #time
                  class="h-10 w-32 rounded-md border border-input px-2"
                  name="arrivalTime"
                  (change)="arrivalTime.set(+time.value)"
                >
                  @for (slot of i.arrival_slots; track slot) {
                    <option [value]="slot" [selected]="slot === chosenTime()">
                      {{ timeText(slot) }}
                    </option>
                  }
                </select>
              </label>
              <label class="flex flex-col gap-1 text-sm">
                {{ t.message }}
                <textarea
                  #message
                  class="min-h-24 rounded-md border border-input p-2"
                  name="message"
                  maxlength="2000"
                  (input)="note.set(message.value)"
                ></textarea>
              </label>
              <!-- Left empty by people; robots fill it. -->
              <input
                #website
                class="hidden"
                tabindex="-1"
                autocomplete="off"
                aria-hidden="true"
                name="website"
                (input)="trap.set(website.value)"
              />
              <label class="flex items-start gap-2 text-sm">
                <input
                  type="checkbox"
                  class="mt-0.5 size-4 accent-primary"
                  name="terms"
                  [checked]="accepted()"
                  (change)="accepted.set(!accepted())"
                />
                <span>
                  {{ t.accept }}
                  <a class="underline" target="_blank" rel="noopener" [href]="termsUrl(i)">{{
                    t.terms
                  }}</a>
                </span>
              </label>
              @if (errors().length > 0) {
                <div role="alert" class="text-sm text-destructive">
                  <p>{{ t.refused }}</p>
                  <ul class="list-inside list-disc">
                    @for (error of errors(); track error) {
                      <li>{{ error }}</li>
                    }
                  </ul>
                </div>
              }
              <div>
                <button
                  type="submit"
                  class="inline-flex h-11 items-center rounded-md bg-primary px-6 font-medium text-primary-foreground hover:opacity-90 disabled:opacity-60"
                  [disabled]="busy()"
                >
                  {{ busy() ? t.sending : t.confirm }}
                </button>
              </div>
            </form>
          }
        }
      } @else if (loadError()) {
        <p role="alert">{{ t.error }}</p>
      }
    </section>
  `,
})
export class BookingPage {
  /** Route data (ADR-0013). */
  readonly lang = input<Lang>('fr');
  /** Query parameters set by the calendar (F004). */
  readonly arrival = input<string | undefined>();
  readonly departure = input<string | undefined>();
  readonly adults = input<string | undefined>();
  readonly children = input<string | undefined>();
  readonly options = input<string | undefined>();

  private readonly api = inject(BookingApi);
  private readonly quotes = inject(AvailabilityApi);
  private readonly router = inject(Router);
  private readonly current = inject(CurrentPage);
  private readonly title = inject(Title);
  private readonly meta = inject(Meta);

  protected readonly guestFields = GUEST_FIELDS;
  protected readonly labels = LABELS;
  protected readonly text = computed(() => BOOKING_TEXT[this.lang()]);
  protected readonly home = computed(() => (this.lang() === 'en' ? '/en' : '/'));
  protected readonly info = signal<BookingInfo | null>(null);
  protected readonly quote = signal<Quote | null>(null);
  protected readonly quoteErrors = signal<string[]>([]);
  protected readonly loadError = signal(false);
  protected readonly guest = signal<GuestDetails>({
    first_name: '',
    last_name: '',
    email: '',
    phone: '+33',
    street: '',
    postcode: '',
    town: '',
    country: '',
  });
  protected readonly arrivalTime = signal<number | null>(null);
  protected readonly note = signal('');
  protected readonly trap = signal('');
  protected readonly accepted = signal(false);
  protected readonly busy = signal(false);
  protected readonly errors = signal<string[]>([]);

  protected readonly adultCount = computed(() => Math.max(Number(this.adults() ?? 2) || 0, 0));
  protected readonly childCount = computed(() => Math.max(Number(this.children() ?? 0) || 0, 0));
  private readonly optionIds = computed(() =>
    (this.options() ?? '')
      .split(',')
      .map(Number)
      .filter((id) => Number.isInteger(id) && id > 0),
  );
  protected readonly chosenOptions = computed(
    () => this.quote()?.options.filter((option) => option.selected) ?? [],
  );
  protected readonly chosenTime = computed(
    () => this.arrivalTime() ?? this.info()?.arrival_slots[0] ?? 0,
  );

  constructor() {
    effect(() => {
      const lang = this.lang();
      const query = new URLSearchParams(this.router.url.split('?')[1] ?? '').toString();
      this.current.alternates.set({
        fr: `/reserver${query ? `?${query}` : ''}`,
        en: `/en/book${query ? `?${query}` : ''}`,
      });
      this.title.setTitle(`${BOOKING_TEXT[lang].title} - La Petite Grange`);
      this.meta.updateTag({ name: 'robots', content: 'noindex' });
      this.guest.update((guest) =>
        guest.country === '' ? { ...guest, country: BOOKING_TEXT[lang].defaultCountry } : guest,
      );
      void untracked(() => this.load());
    });
  }

  protected eur(cents: number): string {
    return money(cents, this.lang());
  }

  protected long(date: string): string {
    return longDate(date, this.lang());
  }

  protected time(minutes: number): string {
    return clock(minutes);
  }

  protected timeText(minutes: number): string {
    return clock(minutes);
  }

  protected termsUrl(info: BookingInfo): string {
    const slug = info.terms?.slug ?? '';
    return this.lang() === 'en' ? `/en/${slug}` : `/${slug}`;
  }

  protected patchGuest(key: keyof GuestDetails, value: string): void {
    this.guest.update((guest) => ({ ...guest, [key]: value }));
  }

  protected async submit(): Promise<void> {
    const arrival = this.arrival();
    const departure = this.departure();
    if (!arrival || !departure) {
      return;
    }
    this.busy.set(true);
    this.errors.set([]);
    try {
      const created = await firstValueFrom(
        this.api.book({
          arrival,
          departure,
          adults: this.adultCount(),
          children: this.childCount(),
          options: this.optionIds(),
          lang: this.lang(),
          guest: this.guest(),
          arrival_time: this.chosenTime(),
          message: this.note(),
          terms_accepted: this.accepted(),
          website: this.trap(),
        }),
      );
      if (created) {
        const path =
          this.lang() === 'en' ? ['/en/booking', created.token] : ['/reservation', created.token];
        await this.router.navigate(path, { queryParams: { confirmed: 1 } });
      }
    } catch (error) {
      this.errors.set(messages(error, this.text().error));
    } finally {
      this.busy.set(false);
    }
  }

  private async load(): Promise<void> {
    const arrival = this.arrival();
    const departure = this.departure();
    if (!arrival || !departure) {
      return;
    }
    try {
      this.info.set(await firstValueFrom(this.api.info(this.lang())));
      this.loadError.set(false);
    } catch {
      this.loadError.set(true);
      return;
    }
    if (!this.info()?.open) {
      return;
    }
    try {
      this.quote.set(
        await firstValueFrom(
          this.quotes.quote({
            arrival,
            departure,
            adults: this.adultCount(),
            children: this.childCount(),
            options: this.optionIds(),
            lang: this.lang(),
          }),
        ),
      );
      this.quoteErrors.set([]);
    } catch (error) {
      this.quote.set(null);
      this.quoteErrors.set(messages(error, this.text().error));
    }
  }
}

function messages(error: unknown, fallback: string): string[] {
  const body = error instanceof HttpErrorResponse ? (error.error as ApiError | null) : null;
  const details = body?.error?.details ?? [];
  if (details.length > 0) {
    return details;
  }
  return [body?.error?.message ?? fallback];
}
