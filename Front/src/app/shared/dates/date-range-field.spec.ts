import { TestBed } from '@angular/core/testing';

import { DateRangeField, DateRangeValue } from './date-range-field';
import { addDays, monthGrid, nightsBetween, parseIso, todayInParis } from './dates';

describe('F005 dates', () => {
  it('parses only real ISO dates', () => {
    expect(parseIso('2027-02-29')).toBeNull();
    expect(parseIso('2028-02-29')?.toISOString()).toBe('2028-02-29T00:00:00.000Z');
    expect(parseIso('10/03/2027')).toBeNull();
  });

  it('counts nights and adds days across summer time', () => {
    expect(nightsBetween('2027-03-27', '2027-03-30')).toBe(3);
    expect(addDays('2027-10-30', 2)).toBe('2027-11-01');
  });

  it('draws months starting on Monday', () => {
    // 1 March 2027 is a Monday.
    expect(monthGrid(2027, 2).slice(0, 2)).toEqual(['2027-03-01', '2027-03-02']);
    // 1 August 2027 is a Sunday: six empty cells first.
    expect(monthGrid(2027, 7).indexOf('2027-08-01')).toBe(6);
  });

  it('knows today in Paris', () => {
    expect(todayInParis(new Date('2026-12-31T23:30:00Z'))).toBe('2027-01-01');
    expect(todayInParis(new Date('2027-07-14T21:30:00Z'))).toBe('2027-07-14');
  });
});

describe('F005 DateRangeField', () => {
  async function render(value: DateRangeValue) {
    const fixture = TestBed.createComponent(DateRangeField);
    const emitted: DateRangeValue[] = [];
    fixture.componentInstance.valueChange.subscribe((next) => {
      emitted.push(next);
      fixture.componentRef.setInput('value', next);
    });
    fixture.componentRef.setInput('value', value);
    await fixture.whenStable();
    const root = fixture.nativeElement as HTMLElement;
    const day = (iso: string) => root.querySelector<HTMLButtonElement>(`[data-date="${iso}"]`)!;
    const [startInput, endInput] = Array.from(root.querySelectorAll<HTMLInputElement>('input'));
    return { fixture, root, emitted, day, startInput, endInput };
  }

  it('F005-S09 calendar clicks fill the date fields', async () => {
    // "Du" already chosen (3 May): a later day sets "Au".
    const { fixture, emitted, day, startInput, endInput } = await render({
      start: '2027-05-03',
      end: null,
    });
    day('2027-05-06').click();
    await fixture.whenStable();
    expect(emitted.at(-1)).toEqual({ start: '2027-05-03', end: '2027-05-06' });
    expect(startInput.value).toBe('2027-05-03');
    expect(endInput.value).toBe('2027-05-06');
    expect(day('2027-05-04').getAttribute('aria-pressed')).toBe('true');

    // A new click once both are set starts a new range.
    day('2027-05-10').click();
    await fixture.whenStable();
    expect(emitted.at(-1)).toEqual({ start: '2027-05-10', end: null });
  });

  it('F005-S09 typing a date moves the highlight and checks the order', async () => {
    const { fixture, root, day, endInput } = await render({
      start: '2027-05-03',
      end: '2027-05-06',
    });
    endInput.value = '2027-05-08';
    endInput.dispatchEvent(new Event('change'));
    await fixture.whenStable();
    expect(day('2027-05-07').getAttribute('aria-pressed')).toBe('true');

    endInput.value = '2027-05-02';
    endInput.dispatchEvent(new Event('change'));
    await fixture.whenStable();
    expect(root.querySelector('[role="alert"]')?.textContent).toContain('« Au » doit être après');
  });

  it('F005-S09 the month arrows move the calendar even when "Du" is chosen', async () => {
    const { fixture, root, day } = await render({ start: '2027-05-03', end: null });
    const arrow = (label: string) =>
      root.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!;
    arrow('Mois suivant').click();
    await fixture.whenStable();
    expect(day('2027-06-15')).not.toBeNull();
    expect(day('2027-05-15')).toBeNull();
    arrow('Mois suivant').click();
    await fixture.whenStable();
    expect(day('2027-07-15')).not.toBeNull();
    // Choosing "Au" in July keeps July on screen.
    day('2027-07-20').click();
    await fixture.whenStable();
    expect(day('2027-07-15')).not.toBeNull();
    arrow('Mois précédent').click();
    await fixture.whenStable();
    expect(day('2027-06-15')).not.toBeNull();
  });

  it('F005-S05 open-ended ranges disable "Au"', async () => {
    const { fixture, endInput, day } = await render({ start: '2027-05-03', end: null });
    fixture.componentRef.setInput('openEnded', true);
    await fixture.whenStable();
    expect(endInput.disabled).toBe(true);
    expect(day('2027-05-30').getAttribute('aria-pressed')).toBe('true');
  });
});
