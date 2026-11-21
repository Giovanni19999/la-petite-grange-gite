import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { todayInParis } from '../../../shared/dates/dates';
import { ClosuresPage } from './closures.page';
import { BlockedPeriod } from './data-access/closures.api';

const URL = '/api/admin/blocked-periods';

/** A day of the month after the current one, so it is always in the future. */
function nextMonthDay(day: number): string {
  const [year, month] = todayInParis().split('-').map(Number);
  const date = new Date(Date.UTC(year, month, day));
  return date.toISOString().slice(0, 10);
}

const FAMILY: BlockedPeriod = {
  id: 7,
  start: nextMonthDay(10),
  end: nextMonthDay(14),
  kind: 'family',
  label: 'Mamie et Papi',
  people: 4,
  reason: null,
  nights: 4,
  updated_at: '2026-11-21T10:00:00Z',
};

describe('F005 closures page', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([{ path: 'admin/fermetures', component: ClosuresPage }]),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
    await harness.navigateByUrl('/admin/fermetures');
    http.expectOne(URL).flush([FAMILY]);
    await settle();
  });

  afterEach(() => http.verify());

  const page = () => harness.routeNativeElement as HTMLElement;
  const button = (text: string, scope: ParentNode = page()) =>
    Array.from(scope.querySelectorAll<HTMLButtonElement>('button')).find(
      (candidate) => candidate.textContent?.trim() === text,
    )!;
  const day = (iso: string) =>
    page().querySelector<HTMLButtonElement>(`form [data-date="${iso}"]`)!;
  const overviewDay = (iso: string) =>
    page().querySelector<HTMLElement>(`[data-testid="overview-month"] [data-date="${iso}"]`);

  async function moveOverviewToNextMonth(): Promise<void> {
    // The overview shows this month and the next: the family stay is already visible.
  }

  it('F005-S02 family periods are shown in the private colour with their label', async () => {
    await moveOverviewToNextMonth();
    const night = overviewDay(nextMonthDay(11))!;
    expect(night.dataset['kind']).toBe('family');
    expect(night.classList).toContain('bg-private');
    expect(overviewDay(nextMonthDay(10))!.textContent).toContain('Mamie et Papi');
    expect(page().querySelector('[data-testid="period-row"]')?.textContent).toContain('4 pers.');
  });

  it('F005-S01 closes dates after confirmation', async () => {
    button('Fermer des dates').click();
    await settle();
    // Move the form calendar to next month.
    button('›', page().querySelector('form')!).click();
    await settle();
    day(nextMonthDay(20)).click();
    await settle();
    day(nextMonthDay(25)).click();
    await settle();
    expect(page().querySelector('[data-testid="nights-summary"]')?.textContent).toContain(
      '(5 nuits)',
    );

    page().querySelector('form')!.dispatchEvent(new Event('submit'));
    await settle();
    expect(document.body.textContent).toContain('Fermer ces dates ?');
    (document.querySelector('[data-slot="alert-dialog-action"]') as HTMLButtonElement).click();

    const request = http.expectOne((req) => req.url === URL && req.method === 'POST');
    expect(request.request.body).toEqual({
      start: nextMonthDay(20),
      end: nextMonthDay(25),
      kind: 'closure',
      label: null,
      people: null,
      reason: null,
    });
    request.flush({ id: 8 }, { status: 201, statusText: 'Created' });
    await settle();
    http.expectOne(URL).flush([
      FAMILY,
      {
        ...FAMILY,
        id: 8,
        kind: 'closure',
        label: null,
        people: null,
        start: nextMonthDay(20),
        end: nextMonthDay(25),
        nights: 5,
      },
    ]);
    await settle();
    expect(page().querySelectorAll('[data-testid="period-row"]')).toHaveLength(2);
  });

  it('F005-S03 a family period needs a label before anything is sent', async () => {
    button('Fermer des dates').click();
    await settle();
    page().querySelector<HTMLInputElement>('form input[type="radio"][value="family"]')!.click();
    await settle();
    const [start, end] = Array.from(
      page().querySelectorAll<HTMLInputElement>('form input[type="date"]'),
    );
    start.value = nextMonthDay(1);
    start.dispatchEvent(new Event('change'));
    await settle();
    end.value = nextMonthDay(3);
    end.dispatchEvent(new Event('change'));
    await settle();
    page().querySelector('form')!.dispatchEvent(new Event('submit'));
    await settle();
    expect(page().querySelector('[data-testid="form-error"]')?.textContent).toContain('libellé');
  });

  it('F005-S06 shows the API refusal of an overlap', async () => {
    overviewDay(nextMonthDay(11))!.click();
    await settle();
    const [, end] = Array.from(
      page().querySelectorAll<HTMLInputElement>('form input[type="date"]'),
    );
    end.value = nextMonthDay(20);
    end.dispatchEvent(new Event('change'));
    await settle();
    page().querySelector('form')!.dispatchEvent(new Event('submit'));
    await settle();
    (document.querySelector('[data-slot="alert-dialog-action"]') as HTMLButtonElement).click();
    http.expectOne(`${URL}/7`).flush(
      {
        error: {
          code: 'period_overlap',
          message: 'ces dates chevauchent une autre période (du …)',
        },
      },
      { status: 409, statusText: 'Conflict' },
    );
    await settle();
    expect(page().querySelector('[data-testid="form-error"]')?.textContent).toContain(
      'chevauchent une autre période',
    );
  });

  it('F005-S07 reopens dates after confirmation', async () => {
    overviewDay(nextMonthDay(11))!.click();
    await settle();
    button('Rouvrir ces dates').click();
    await settle();
    expect(document.body.textContent).toContain('Rouvrir ces dates ?');
    (document.querySelector('[data-slot="alert-dialog-action"]') as HTMLButtonElement).click();
    const request = http.expectOne(`${URL}/7`);
    expect(request.request.method).toBe('DELETE');
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    http.expectOne(URL).flush([]);
    await settle();
    expect(page().querySelectorAll('[data-testid="period-row"]')).toHaveLength(0);
  });
});

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
