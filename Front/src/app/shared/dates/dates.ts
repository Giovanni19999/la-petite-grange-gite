/**
 * Calendar dates as ISO strings (`2027-03-10`), the format of the API. Computed in UTC so
 * that the visitor's time zone and summer time never shift a day.
 */
export type IsoDate = string;

const ISO = /^(\d{4})-(\d{2})-(\d{2})$/;

/** The UTC midnight of an ISO date, or `null` if it is not a real date. */
export function parseIso(value: string | null | undefined): Date | null {
  const match = ISO.exec(value ?? '');
  if (!match) {
    return null;
  }
  const [year, month, day] = [Number(match[1]), Number(match[2]), Number(match[3])];
  const date = new Date(Date.UTC(year, month - 1, day));
  return date.getUTCFullYear() === year &&
    date.getUTCMonth() === month - 1 &&
    date.getUTCDate() === day
    ? date
    : null;
}

export function toIso(date: Date): IsoDate {
  return date.toISOString().slice(0, 10);
}

export function addDays(value: IsoDate, days: number): IsoDate {
  const date = parseIso(value)!;
  date.setUTCDate(date.getUTCDate() + days);
  return toIso(date);
}

/** Number of nights from `start` to `end` (excluded). */
export function nightsBetween(start: IsoDate, end: IsoDate): number {
  return Math.round((parseIso(end)!.getTime() - parseIso(start)!.getTime()) / 86_400_000);
}

/** Today in Europe/Paris, the cottage's time zone. */
export function todayInParis(now = new Date()): IsoDate {
  return new Intl.DateTimeFormat('en-CA', {
    timeZone: 'Europe/Paris',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).format(now);
}

/** "10/03/2027". */
export function formatShort(value: IsoDate): string {
  const [year, month, day] = value.split('-');
  return `${day}/${month}/${year}`;
}

/** "mercredi 10 mars 2027" (or "10 mars" without year and weekday). */
export function formatLong(
  value: IsoDate,
  options: { weekday?: boolean; year?: boolean } = {},
): string {
  return new Intl.DateTimeFormat('fr-FR', {
    timeZone: 'UTC',
    weekday: options.weekday === false ? undefined : 'long',
    day: 'numeric',
    month: 'long',
    year: options.year === false ? undefined : 'numeric',
  }).format(parseIso(value)!);
}

/** "mars 2027". */
export function formatMonth(year: number, month: number): string {
  return new Intl.DateTimeFormat('fr-FR', {
    timeZone: 'UTC',
    month: 'long',
    year: 'numeric',
  }).format(new Date(Date.UTC(year, month, 1)));
}

/** The days to draw for a month (`month` 0-11), weeks starting on Monday; `null` pads. */
export function monthGrid(year: number, month: number): (IsoDate | null)[] {
  const first = new Date(Date.UTC(year, month, 1));
  const daysInMonth = new Date(Date.UTC(year, month + 1, 0)).getUTCDate();
  const padding = (first.getUTCDay() + 6) % 7;
  const cells: (IsoDate | null)[] = Array.from({ length: padding }, () => null);
  for (let day = 1; day <= daysInMonth; day++) {
    cells.push(toIso(new Date(Date.UTC(year, month, day))));
  }
  while (cells.length % 7 !== 0) {
    cells.push(null);
  }
  return cells;
}

export const WEEKDAYS_SHORT = ['lun.', 'mar.', 'mer.', 'jeu.', 'ven.', 'sam.', 'dim.'] as const;

/** Month after (`delta` 1) or before (-1) the given one. */
export function shiftMonth(
  year: number,
  month: number,
  delta: number,
): { year: number; month: number } {
  const date = new Date(Date.UTC(year, month + delta, 1));
  return { year: date.getUTCFullYear(), month: date.getUTCMonth() };
}
