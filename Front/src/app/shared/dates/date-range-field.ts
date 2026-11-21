import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  input,
  output,
  signal,
} from '@angular/core';

import {
  IsoDate,
  WEEKDAYS_SHORT,
  formatLong,
  formatMonth,
  monthGrid,
  parseIso,
  shiftMonth,
  todayInParis,
} from './dates';

/** A range of nights: `end` excluded (the day after the last night), `null` when not chosen. */
export interface DateRangeValue {
  start: IsoDate | null;
  end: IsoDate | null;
}

let nextId = 0;

/**
 * Shared date range field (plan "DateRangeField"): a month calendar and two date fields kept
 * in sync both ways. Clicking a day sets "Du", a later day sets "Au"; typing a date moves the
 * calendar. Validation as you type: "Au" after "Du". Used for closures (F005), prices (F006)
 * and stays (F004).
 */
@Component({
  selector: 'app-date-range-field',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="flex flex-col gap-3" role="group" [attr.aria-labelledby]="id + '-legend'">
      <span class="text-sm font-medium" [id]="id + '-legend'">{{ legend() }}</span>
      <div class="grid grid-cols-2 gap-3">
        <label class="flex flex-col gap-1 text-sm">
          {{ startLabel() }}
          <input
            #startInput
            class="h-9 rounded-md border border-input bg-background px-2"
            type="date"
            [value]="value().start ?? ''"
            (change)="typeStart(startInput.value)"
          />
        </label>
        <label class="flex flex-col gap-1 text-sm">
          {{ endLabel() }}
          <input
            #endInput
            class="h-9 rounded-md border border-input bg-background px-2 disabled:opacity-50"
            type="date"
            [value]="value().end ?? ''"
            [min]="value().start ?? ''"
            [disabled]="openEnded()"
            [attr.aria-invalid]="error() ? 'true' : null"
            (change)="typeEnd(endInput.value)"
          />
        </label>
      </div>
      @if (error(); as message) {
        <p class="text-sm text-destructive" role="alert">{{ message }}</p>
      }

      <div class="rounded-md border p-2">
        <div class="mb-2 flex items-center justify-between">
          <button
            type="button"
            class="rounded px-2 py-1 hover:bg-muted"
            aria-label="Mois précédent"
            (click)="move(-1)"
          >
            ‹
          </button>
          <span class="text-sm font-medium capitalize" aria-live="polite">{{ monthTitle() }}</span>
          <button
            type="button"
            class="rounded px-2 py-1 hover:bg-muted"
            aria-label="Mois suivant"
            (click)="move(1)"
          >
            ›
          </button>
        </div>
        <div
          class="grid grid-cols-7 gap-1 text-center text-xs text-muted-foreground"
          aria-hidden="true"
        >
          @for (day of weekdays; track day) {
            <span>{{ day }}</span>
          }
        </div>
        <div class="mt-1 grid grid-cols-7 gap-1">
          @for (day of days(); track $index) {
            @if (day) {
              <button
                type="button"
                class="aspect-square rounded text-sm hover:bg-muted aria-pressed:font-semibold"
                [class.bg-primary]="isEdge(day)"
                [class.text-primary-foreground]="isEdge(day)"
                [class.bg-primary/15]="isInside(day)"
                [class.ring-1]="day === today"
                [class.ring-ring]="day === today"
                [attr.aria-pressed]="isEdge(day) || isInside(day)"
                [attr.aria-label]="label(day)"
                [attr.data-date]="day"
                (click)="pick(day)"
              >
                {{ dayNumber(day) }}
              </button>
            } @else {
              <span></span>
            }
          }
        </div>
      </div>
    </div>
  `,
})
export class DateRangeField {
  readonly value = input<DateRangeValue>({ start: null, end: null });
  /** No end date ("jusqu'à nouvel ordre"): the "Au" field is disabled. */
  readonly openEnded = input(false);
  readonly legend = input('Dates');
  readonly startLabel = input('Du');
  readonly endLabel = input('Au');
  readonly valueChange = output<DateRangeValue>();

  protected readonly id = `range-${nextId++}`;
  protected readonly weekdays = WEEKDAYS_SHORT;
  protected readonly today = todayInParis();
  private readonly shown = signal(monthOf(this.today));

  protected readonly days = computed(() => monthGrid(this.shown().year, this.shown().month));
  protected readonly monthTitle = computed(() =>
    formatMonth(this.shown().year, this.shown().month),
  );
  protected readonly error = computed(() => {
    const { start, end } = this.value();
    return start && end && !this.openEnded() && end <= start
      ? '« Au » doit être après « Du » (le jour du départ ou du lendemain de la dernière nuit).'
      : null;
  });

  constructor() {
    // Typing or loading a date shows its month.
    effect(() => {
      const start = this.value().start;
      if (start && parseIso(start)) {
        const month = monthOf(start);
        const shown = this.shown();
        if (month.year !== shown.year || month.month !== shown.month) {
          this.shown.set(month);
        }
      }
    });
  }

  protected move(delta: number): void {
    this.shown.update(({ year, month }) => shiftMonth(year, month, delta));
  }

  /** First click: "Du"; a later day: "Au"; a day before "Du" restarts the range. */
  protected pick(day: IsoDate): void {
    const { start, end } = this.value();
    if (this.openEnded()) {
      this.valueChange.emit({ start: day, end: null });
    } else if (!start || end || day <= start) {
      this.valueChange.emit({ start: day, end: null });
    } else {
      this.valueChange.emit({ start, end: day });
    }
  }

  protected typeStart(value: string): void {
    this.valueChange.emit({ start: parseIso(value) ? value : null, end: this.value().end });
  }

  protected typeEnd(value: string): void {
    this.valueChange.emit({ start: this.value().start, end: parseIso(value) ? value : null });
  }

  protected isEdge(day: IsoDate): boolean {
    const { start, end } = this.value();
    return day === start || (!this.openEnded() && day === end);
  }

  protected isInside(day: IsoDate): boolean {
    const { start, end } = this.value();
    if (!start || day <= start) {
      return false;
    }
    if (this.openEnded()) {
      return true;
    }
    return end !== null && day < end;
  }

  protected dayNumber(day: IsoDate): number {
    return Number(day.slice(8));
  }

  protected label(day: IsoDate): string {
    return formatLong(day);
  }
}

function monthOf(day: IsoDate): { year: number; month: number } {
  const date = parseIso(day) ?? parseIso(todayInParis())!;
  return { year: date.getUTCFullYear(), month: date.getUTCMonth() };
}
