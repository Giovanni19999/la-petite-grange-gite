import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef, ComponentRef } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

import { IsoDate, addDays, todayInParis } from '../../../../../shared/dates/dates';
import { CalendarNight, PublicCalendar } from '../../data-access/availability.api';
import { AvailabilityBlock } from '../../data-access/pages.models';
import { AvailabilityBlockView } from './availability-block';

const TODAY = todayInParis();
/** Free from tomorrow at 100 €, except a closed week from day 20 to day 26. */
const CLOSED = (day: IsoDate) => day >= addDays(TODAY, 20) && day < addDays(TODAY, 27);

function block(months = 2): AvailabilityBlock {
  return {
    id: 'a',
    type: 'availability',
    background: 'background',
    spacing: 'medium',
    hide_on_mobile: false,
    months,
  };
}

function calendarFor(from: IsoDate, to: IsoDate): PublicCalendar {
  const nights: CalendarNight[] = [];
  for (let day = from; day < to; day = addDays(day, 1)) {
    const free = day > TODAY && !CLOSED(day);
    nights.push(free ? { date: day, price: 10_000, min_nights: 2 } : { date: day, price: null });
  }
  return {
    earliest_arrival: addDays(TODAY, 1),
    last_night: addDays(TODAY, 700),
    capacity: 5,
    children_under: 18,
    arrival_weekdays: [1, 2, 3, 4, 5, 6, 7],
    departure_weekdays: [1, 2, 3, 4, 5, 6, 7],
    booking_open: true,
    nights,
  };
}

describe('F004 availability block', () => {
  let fixture: ComponentFixture<AvailabilityBlockView>;
  let http: HttpTestingController;

  async function setup(months = 2): Promise<void> {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting(), provideRouter([])],
    });
    http = TestBed.inject(HttpTestingController);
    fixture = TestBed.createComponent(AvailabilityBlockView);
    (fixture.componentRef as ComponentRef<AvailabilityBlockView>).setInput('block', block(months));
    await settle();
    flushCalendar();
    await settle();
  }

  function flushCalendar(): void {
    const request = http.expectOne((req) => req.url === '/api/calendar');
    const params = request.request.params;
    request.flush(calendarFor(params.get('from')!, params.get('to')!));
  }

  afterEach(() => http.verify());

  const root = () => fixture.nativeElement as HTMLElement;
  const dayButton = (day: IsoDate) =>
    root().querySelector<HTMLButtonElement>(`[data-date="${day}"]`)!;
  const field = (name: string) => root().querySelector<HTMLInputElement>(`[name="${name}"]`)!;
  async function type(name: string, value: string, event = 'change'): Promise<void> {
    field(name).value = value;
    field(name).dispatchEvent(new Event(event));
    await settle();
  }

  it('F004-S01 free nights show their price, F004-S02 unavailable nights show no detail', async () => {
    await setup();
    const free = addDays(TODAY, 10);
    expect(dayButton(free).textContent).toContain('100');
    expect(dayButton(free).getAttribute('aria-label')).toContain('libre');
    const closed = addDays(TODAY, 21);
    if (dayButton(closed)) {
      expect(dayButton(closed).querySelector('[data-testid="price"]')).toBeNull();
      expect(dayButton(closed).getAttribute('aria-label')).toMatch(/indisponible$/);
    }
    expect(dayButton(TODAY).getAttribute('aria-label')).toMatch(/indisponible$/);
  });

  it('F004-S03 F004-S06 typed dates select the stay and show the quote from the API', async () => {
    await setup();
    const arrival = addDays(TODAY, 5);
    const departure = addDays(TODAY, 8);
    await type('arrival', arrival);
    await type('departure', departure);
    const request = http.expectOne('/api/quote');
    expect(request.request.body).toEqual({
      arrival,
      departure,
      adults: 2,
      children: 0,
      options: [],
      lang: 'fr',
    });
    request.flush({
      nights: 3,
      nights_total: 30_000,
      promotion: null,
      cleaning_fee: 6_000,
      options: [],
      options_total: 0,
      stay_total: 36_000,
      deposit: 36_000,
      balance: 0,
      balance_due: null,
      tourist_tax: 1_122,
    });
    await settle();
    expect(dayButton(addDays(TODAY, 6)).getAttribute('aria-pressed')).toBe('true');
    expect(dayButton(departure).getAttribute('aria-pressed')).toBe('true');
    const quote = root().querySelector('[data-testid="quote"]')!.textContent!;
    expect(quote).toContain('3 nuits');
    expect(root().querySelector('[data-testid="stay-total"]')!.textContent).toMatch(/360/);
    expect(quote).toContain('La totalité est due à la réservation.');
    expect(root().querySelector('[data-testid="tourist-tax"]')!.textContent).toMatch(/11,22/);
    // F007: the quote leads to the booking page with the stay.
    const book = root().querySelector<HTMLAnchorElement>('[data-testid="book"]')!;
    expect(book.getAttribute('href')).toBe(
      `/reserver?arrival=${arrival}&departure=${departure}&adults=2&children=0`,
    );
  });

  it('F026-S01 F026-S04 shows the promotion and lets the visitor tick options', async () => {
    await setup();
    const arrival = addDays(TODAY, 5);
    const departure = addDays(TODAY, 12);
    await type('arrival', arrival);
    await type('departure', departure);
    const quoteOf = (selected: boolean) => ({
      nights: 7,
      nights_total: 70_000,
      promotion: { name: 'Semaine', discount: 7_000 },
      cleaning_fee: 6_000,
      options: [
        {
          id: 4,
          name: 'Linge de maison',
          description: 'Draps et serviettes',
          price: 3_000,
          selected,
        },
        { id: 5, name: 'Lit bébé', description: '', price: 0, selected: false },
      ],
      options_total: selected ? 3_000 : 0,
      stay_total: selected ? 72_000 : 69_000,
      deposit: 36_000,
      balance: 36_000,
      balance_due: addDays(arrival, -30),
      tourist_tax: 0,
    });
    http.expectOne('/api/quote').flush(quoteOf(false));
    await settle();
    expect(root().querySelector('[data-testid="promotion"]')!.textContent).toContain(
      'Promotion « Semaine »',
    );
    const options = root().querySelector('[data-testid="options"]')!;
    expect(options.textContent).toContain('Lit bébé');
    expect(options.textContent).toContain('offert');

    options.querySelector<HTMLInputElement>('[data-option="4"]')!.click();
    await settle();
    const request = http.expectOne('/api/quote');
    expect(request.request.body.options).toEqual([4]);
    request.flush(quoteOf(true));
    await settle();
    expect(root().querySelector('[data-testid="stay-total"]')!.textContent).toMatch(/720/);

    // Other dates: the ticked options are cleared.
    await type('departure', addDays(TODAY, 13));
    expect(http.expectOne('/api/quote').request.body.options).toEqual([]);
  });

  it('F004-S04 refusals from the API are shown', async () => {
    await setup();
    dayButton(addDays(TODAY, 5)).click();
    await settle();
    expect(root().textContent).toContain('Choisissez maintenant votre date de départ.');
    dayButton(addDays(TODAY, 6)).click();
    await settle();
    http.expectOne('/api/quote').flush(
      {
        error: {
          code: 'invalid',
          message: 'Contenu invalide',
          details: ['2 nuits minimum pour cette date d’arrivée'],
        },
      },
      { status: 422, statusText: 'Unprocessable Entity' },
    );
    await settle();
    expect(root().querySelector('[role="alert"]')!.textContent).toContain('2 nuits minimum');
  });

  it('F004-S07 one month on a phone, arrow keys move between days', async () => {
    await setup(2);
    const months = root().querySelectorAll<HTMLElement>('[data-testid="month"]');
    expect(months.length).toBe(2);
    expect(months[1].className).toContain('@max-3xl:hidden');
    const start = addDays(TODAY, 1);
    dayButton(start).focus();
    await settle();
    dayButton(start).dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }),
    );
    await settle();
    // Moving past the shown months loads the next ones.
    const moved = addDays(start, 1);
    if (!dayButton(moved)) {
      flushCalendar();
      await settle();
    }
    expect(dayButton(moved).tabIndex).toBe(0);
    expect(dayButton(start).tabIndex).toBe(-1);
  });

  it('F004-S07 the month buttons change the shown months', async () => {
    await setup(1);
    const previous = root().querySelector<HTMLButtonElement>('[aria-label="Mois précédent"]')!;
    expect(previous.disabled).toBe(true);
    root().querySelector<HTMLButtonElement>('[aria-label="Mois suivant"]')!.click();
    await settle();
    flushCalendar();
    await settle();
    expect(previous.disabled).toBe(false);
  });
});

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
