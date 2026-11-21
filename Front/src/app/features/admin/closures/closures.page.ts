import { DatePipe } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { HlmInputImports } from '@spartan-ng/helm/input';
import { HlmLabelImports } from '@spartan-ng/helm/label';
import { firstValueFrom } from 'rxjs';

import { Choice, ChoiceGroup } from '../../../shared/choice/choice-group';
import { ConfirmDialog } from '../../../shared/confirm/confirm-dialog';
import { DateRangeField, DateRangeValue } from '../../../shared/dates/date-range-field';
import {
  IsoDate,
  WEEKDAYS_SHORT,
  addDays,
  formatLong,
  formatMonth,
  formatShort,
  monthGrid,
  nightsBetween,
  shiftMonth,
  todayInParis,
} from '../../../shared/dates/dates';
import { ApiErrorBody } from '../pages/data-access/admin-pages.models';
import {
  BlockedKind,
  BlockedPeriod,
  BlockedPeriodInput,
  ClosuresApi,
  KIND_LABELS,
  PeriodEvent,
} from './data-access/closures.api';

const KINDS: readonly Choice<BlockedKind>[] = [
  { value: 'closure', label: KIND_LABELS.closure },
  { value: 'family', label: KIND_LABELS.family },
  { value: 'private_event', label: KIND_LABELS.private_event },
];

/** Months shown side by side in the overview calendar. */
const OVERVIEW_MONTHS = 2;

interface FormState {
  id: number | null;
  range: DateRangeValue;
  untilFurtherNotice: boolean;
  kind: BlockedKind;
  label: string;
  people: string;
  reason: string;
}

function emptyForm(): FormState {
  return {
    id: null,
    range: { start: null, end: null },
    untilFurtherNotice: false,
    kind: 'closure',
    label: '',
    people: '',
    reason: '',
  };
}

/** "Fermetures" (F005): close dates for works, family or private events. */
@Component({
  selector: 'app-closures-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    DatePipe,
    RouterLink,
    HlmButtonImports,
    HlmInputImports,
    HlmLabelImports,
    ChoiceGroup,
    ConfirmDialog,
    DateRangeField,
  ],
  template: `
    <section class="mx-auto flex max-w-5xl flex-col gap-6 px-4 py-8">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <a routerLink="/admin" class="text-sm text-muted-foreground hover:underline">
            ← Espace administrateur
          </a>
          <h1 class="text-2xl font-semibold">Fermetures</h1>
        </div>
        <button hlmBtn type="button" (click)="startCreate()">Fermer des dates</button>
      </div>

      <p class="rounded-md bg-muted p-3 text-sm text-muted-foreground">
        Les dates fermées ne peuvent plus être réservées sur le site. Sans channel manager, Airbnb
        et Booking lisent le calendrier toutes les 30 minutes à quelques heures : pour une fermeture
        urgente, fermez aussi les dates directement sur ces plateformes.
      </p>

      @if (loadError()) {
        <p class="text-destructive" role="alert">Impossible de charger les fermetures.</p>
      }

      @if (form(); as f) {
        <form
          class="flex flex-col gap-4 rounded-lg border p-4"
          [attr.aria-label]="f.id ? 'Modifier la période' : 'Nouvelle période'"
          (submit)="$event.preventDefault(); askSave()"
        >
          <h2 class="font-semibold">{{ f.id ? 'Modifier la période' : 'Nouvelle période' }}</h2>
          <app-choice-group
            legend="Type"
            [choices]="kinds"
            [value]="f.kind"
            (changed)="patch({ kind: $event })"
          />
          <app-date-range-field
            legend="Dates"
            startLabel="Du (première nuit fermée)"
            endLabel="Au (lendemain de la dernière nuit)"
            [value]="f.range"
            [openEnded]="f.untilFurtherNotice"
            (valueChange)="patch({ range: $event })"
          />
          <label class="flex items-center gap-2 text-sm">
            <input
              #until
              type="checkbox"
              class="size-4 accent-primary"
              [checked]="f.untilFurtherNotice"
              (change)="patch({ untilFurtherNotice: until.checked })"
            />
            Jusqu'à nouvel ordre (aucune date de fin)
          </label>
          @if (nightsText(); as text) {
            <p class="text-sm" data-testid="nights-summary">{{ text }}</p>
          }
          @if (f.kind !== 'closure') {
            <div class="grid gap-3 sm:grid-cols-[1fr_8rem]">
              <label hlmLabel class="flex flex-col items-start gap-2">
                Libellé (visible seulement ici)
                <input
                  #label
                  hlmInput
                  class="w-full"
                  maxlength="80"
                  placeholder="Ex. : Mamie et Papi"
                  [value]="f.label"
                  (input)="patch({ label: label.value })"
                />
              </label>
              <label hlmLabel class="flex flex-col items-start gap-2">
                Personnes
                <input
                  #people
                  hlmInput
                  class="w-full"
                  type="number"
                  min="1"
                  max="99"
                  [value]="f.people"
                  (input)="patch({ people: people.value })"
                />
              </label>
            </div>
          }
          <label hlmLabel class="flex flex-col items-start gap-2">
            Motif ou remarque (facultatif)
            <input
              #reason
              hlmInput
              class="w-full"
              maxlength="300"
              placeholder="Ex. : travaux de peinture"
              [value]="f.reason"
              (input)="patch({ reason: reason.value })"
            />
          </label>
          @if (formError(); as message) {
            <p class="text-sm text-destructive" role="alert" data-testid="form-error">
              {{ message }}
            </p>
          }
          <div class="flex flex-wrap gap-2">
            <button hlmBtn type="submit" [disabled]="busy()">Enregistrer</button>
            <button hlmBtn variant="outline" type="button" (click)="form.set(null)">Annuler</button>
            @if (f.id) {
              <button
                hlmBtn
                variant="ghost"
                type="button"
                class="ml-auto text-destructive"
                (click)="askDelete()"
              >
                Rouvrir ces dates
              </button>
            }
          </div>
        </form>
      }

      <div class="flex flex-col gap-3">
        <div class="flex items-center justify-between">
          <button
            type="button"
            class="rounded px-2 py-1 hover:bg-muted"
            aria-label="Mois précédents"
            (click)="moveOverview(-1)"
          >
            ‹
          </button>
          <div class="flex flex-wrap gap-3 text-xs" aria-label="Légende">
            <span class="flex items-center gap-1"
              ><span class="size-3 rounded-sm bg-muted-foreground/60"></span>Fermeture</span
            >
            <span class="flex items-center gap-1"
              ><span class="size-3 rounded-sm bg-private"></span>Famille, événement privé</span
            >
          </div>
          <button
            type="button"
            class="rounded px-2 py-1 hover:bg-muted"
            aria-label="Mois suivants"
            (click)="moveOverview(1)"
          >
            ›
          </button>
        </div>
        <div class="grid gap-4 md:grid-cols-2">
          @for (month of months(); track month.title) {
            <div class="rounded-lg border p-3" data-testid="overview-month">
              <h2 class="mb-2 text-center text-sm font-medium capitalize">{{ month.title }}</h2>
              <div
                class="grid grid-cols-7 gap-1 text-center text-xs text-muted-foreground"
                aria-hidden="true"
              >
                @for (day of weekdays; track day) {
                  <span>{{ day }}</span>
                }
              </div>
              <div class="mt-1 grid grid-cols-7 gap-1">
                @for (day of month.days; track $index) {
                  @if (day) {
                    @let period = periodOn(day);
                    <button
                      type="button"
                      class="flex aspect-square flex-col items-center justify-center overflow-hidden rounded text-xs"
                      [class.bg-muted-foreground/60]="period?.kind === 'closure'"
                      [class.text-background]="period?.kind === 'closure'"
                      [class.bg-private]="period !== null && period.kind !== 'closure'"
                      [class.text-private-foreground]="period !== null && period.kind !== 'closure'"
                      [class.hover:bg-muted]="period === null"
                      [class.ring-1]="day === today"
                      [class.ring-ring]="day === today"
                      [attr.aria-label]="dayLabel(day, period)"
                      [attr.data-date]="day"
                      [attr.data-kind]="period?.kind ?? null"
                      (click)="period ? startEdit(period) : startCreate(day)"
                    >
                      <span>{{ dayNumber(day) }}</span>
                      @if (period?.label && day === firstShownNight(period!, month.days)) {
                        <span class="w-full truncate px-0.5 text-[0.6rem]">{{
                          period!.label
                        }}</span>
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
      </div>

      <div class="flex flex-col gap-2">
        <div class="flex items-center justify-between gap-2">
          <h2 class="font-semibold">Périodes {{ showPast() ? '' : 'en cours et à venir' }}</h2>
          <label class="flex items-center gap-2 text-sm">
            <input
              #past
              type="checkbox"
              class="size-4 accent-primary"
              [checked]="showPast()"
              (change)="showPast.set(past.checked)"
            />
            Afficher les périodes passées
          </label>
        </div>
        <ul class="flex flex-col divide-y rounded-lg border" aria-label="Périodes fermées">
          @for (period of visiblePeriods(); track period.id) {
            <li class="flex flex-wrap items-center gap-3 p-3" data-testid="period-row">
              <span
                class="rounded-full px-2 py-0.5 text-xs"
                [class.bg-muted]="period.kind === 'closure'"
                [class.bg-private]="period.kind !== 'closure'"
                [class.text-private-foreground]="period.kind !== 'closure'"
                >{{ kindLabel(period.kind) }}</span
              >
              <div class="flex min-w-0 flex-1 flex-col">
                <span class="font-medium">{{ describe(period) }}</span>
                <span class="text-sm text-muted-foreground">
                  {{ period.label ?? '' }}{{ period.people ? ' · ' + period.people + ' pers.' : ''
                  }}{{ period.reason ? ' · ' + period.reason : '' }}
                </span>
              </div>
              <button hlmBtn variant="outline" size="sm" type="button" (click)="startEdit(period)">
                Modifier
              </button>
            </li>
          } @empty {
            <li class="p-3 text-sm text-muted-foreground">Aucune période fermée.</li>
          }
        </ul>
      </div>

      <details class="rounded-lg border p-3" (toggle)="loadHistory()">
        <summary class="cursor-pointer font-semibold">Historique des modifications</summary>
        <ul class="mt-2 flex flex-col gap-1 text-sm" aria-label="Historique">
          @for (event of history(); track $index) {
            <li>
              <span class="font-medium">{{ actionLabel(event) }}</span>
              {{ eventDates(event) }} · {{ event.author ?? 'compte supprimé' }} ·
              <span class="text-muted-foreground">{{ event.at | date: 'dd/MM/yyyy HH:mm' }}</span>
            </li>
          } @empty {
            <li class="text-muted-foreground">Aucune modification.</li>
          }
        </ul>
      </details>
    </section>

    <app-confirm-dialog
      [open]="confirm() !== null"
      [title]="confirm()?.title ?? ''"
      [description]="confirm()?.description ?? ''"
      [confirmLabel]="confirm()?.action ?? 'Confirmer'"
      [destructive]="confirm()?.kind === 'delete'"
      (confirmed)="runConfirmed()"
      (cancelled)="confirm.set(null)"
    />
  `,
})
export class ClosuresPage {
  private readonly api = inject(ClosuresApi);

  protected readonly kinds = KINDS;
  protected readonly weekdays = WEEKDAYS_SHORT;
  protected readonly today = todayInParis();
  protected readonly periods = signal<BlockedPeriod[]>([]);
  protected readonly history = signal<PeriodEvent[]>([]);
  protected readonly loadError = signal(false);
  protected readonly form = signal<FormState | null>(null);
  protected readonly formError = signal<string | null>(null);
  protected readonly busy = signal(false);
  protected readonly showPast = signal(false);
  protected readonly confirm = signal<{
    kind: 'save' | 'delete';
    title: string;
    description: string;
    action: string;
  } | null>(null);
  private readonly overview = signal(monthOf(this.today));

  protected readonly months = computed(() => {
    const { year, month } = this.overview();
    return Array.from({ length: OVERVIEW_MONTHS }, (_, index) => {
      const shown = shiftMonth(year, month, index);
      return {
        title: formatMonth(shown.year, shown.month),
        days: monthGrid(shown.year, shown.month),
      };
    });
  });
  protected readonly visiblePeriods = computed(() =>
    this.periods().filter(
      (period) => this.showPast() || period.end === null || period.end > this.today,
    ),
  );
  protected readonly nightsText = computed(() => {
    const f = this.form();
    const start = f?.range.start;
    if (!f || !start) {
      return null;
    }
    if (f.untilFurtherNotice) {
      return `Toutes les nuits à partir du ${formatLong(start, { weekday: false })}.`;
    }
    const end = f.range.end;
    if (!end || end <= start) {
      return null;
    }
    const nights = nightsBetween(start, end);
    return `Nuits fermées : du ${formatLong(start, { weekday: false })} au ${formatLong(addDays(end, -1), { weekday: false })} (${nights} nuit${nights > 1 ? 's' : ''}).`;
  });

  constructor() {
    void this.reload();
  }

  protected dayNumber(day: IsoDate): number {
    return Number(day.slice(8));
  }

  protected kindLabel(kind: BlockedKind): string {
    return KIND_LABELS[kind];
  }

  protected describe(period: BlockedPeriodInput): string {
    return period.end === null
      ? `À partir du ${formatShort(period.start)} (jusqu'à nouvel ordre)`
      : `Du ${formatShort(period.start)} au ${formatShort(period.end)} · ${nightsBetween(period.start, period.end)} nuit(s)`;
  }

  protected periodOn(day: IsoDate): BlockedPeriod | null {
    return (
      this.periods().find(
        (period) => day >= period.start && (period.end === null || day < period.end),
      ) ?? null
    );
  }

  /** The label is written on the first night of the period shown in a month. */
  protected firstShownNight(period: BlockedPeriod, days: (IsoDate | null)[]): IsoDate | null {
    return (
      days.find(
        (day) => day !== null && day >= period.start && (period.end === null || day < period.end),
      ) ?? null
    );
  }

  protected dayLabel(day: IsoDate, period: BlockedPeriod | null): string {
    const date = formatLong(day);
    return period
      ? `${date} : ${KIND_LABELS[period.kind]}${period.label ? ' (' + period.label + ')' : ''}`
      : `${date} : libre, fermer à partir de ce jour`;
  }

  protected moveOverview(delta: number): void {
    this.overview.update(({ year, month }) => shiftMonth(year, month, delta * OVERVIEW_MONTHS));
  }

  protected startCreate(day?: IsoDate): void {
    this.formError.set(null);
    this.form.set({ ...emptyForm(), range: { start: day ?? null, end: null } });
  }

  protected startEdit(period: BlockedPeriod): void {
    this.formError.set(null);
    this.form.set({
      id: period.id,
      range: { start: period.start, end: period.end },
      untilFurtherNotice: period.end === null,
      kind: period.kind,
      label: period.label ?? '',
      people: period.people?.toString() ?? '',
      reason: period.reason ?? '',
    });
  }

  protected patch(changes: Partial<FormState>): void {
    this.form.update((form) => (form ? { ...form, ...changes } : form));
  }

  /** Checks the form, then asks for confirmation with the nights concerned. */
  protected askSave(): void {
    const f = this.form();
    const input = f && this.toInput(f);
    if (!f || !input) {
      return;
    }
    this.confirm.set({
      kind: 'save',
      title: f.id ? 'Modifier cette période ?' : 'Fermer ces dates ?',
      description: `${this.describe(input)}. Les visiteurs ne pourront plus réserver ces nuits.`,
      action: 'Confirmer',
    });
  }

  protected askDelete(): void {
    const f = this.form();
    if (!f?.id) {
      return;
    }
    this.confirm.set({
      kind: 'delete',
      title: 'Rouvrir ces dates ?',
      description: 'La période est supprimée : ces nuits redeviennent réservables.',
      action: 'Rouvrir',
    });
  }

  protected async runConfirmed(): Promise<void> {
    const pending = this.confirm();
    const f = this.form();
    this.confirm.set(null);
    if (!pending || !f) {
      return;
    }
    this.busy.set(true);
    this.formError.set(null);
    try {
      if (pending.kind === 'delete' && f.id) {
        await firstValueFrom(this.api.delete(f.id));
      } else {
        const input = this.toInput(f);
        if (!input) {
          return;
        }
        if (f.id) {
          await firstValueFrom(this.api.update(f.id, input));
        } else {
          await firstValueFrom(this.api.create(input));
        }
      }
      this.form.set(null);
      await this.reload();
      if (this.history().length > 0) {
        await this.loadHistory();
      }
    } catch (error) {
      this.formError.set(errorMessage(error));
    } finally {
      this.busy.set(false);
    }
  }

  protected async loadHistory(): Promise<void> {
    try {
      this.history.set(await firstValueFrom(this.api.history()));
    } catch {
      this.history.set([]);
    }
  }

  protected actionLabel(event: PeriodEvent): string {
    return { created: 'Créée', updated: 'Modifiée', deleted: 'Supprimée' }[event.action];
  }

  protected eventDates(event: PeriodEvent): string {
    const period = event.after ?? event.before;
    if (!period) {
      return '';
    }
    const before =
      event.action === 'updated' && event.before ? ` (avant : ${this.describe(event.before)})` : '';
    return `${KIND_LABELS[period.kind]} ${this.describe(period).toLowerCase()}${before}`;
  }

  /** The form as the API expects it, or `null` (with a message) when incomplete. */
  private toInput(f: FormState): BlockedPeriodInput | null {
    const { start, end } = f.range;
    if (!start || (!f.untilFurtherNotice && (!end || end <= start))) {
      this.formError.set('Choisissez la première nuit fermée et le lendemain de la dernière nuit.');
      return null;
    }
    if (f.kind !== 'closure' && !f.label.trim()) {
      this.formError.set('Donnez un libellé à cette période (visible seulement dans l’admin).');
      return null;
    }
    const people = Number(f.people);
    return {
      start,
      end: f.untilFurtherNotice ? null : end,
      kind: f.kind,
      label: f.kind === 'closure' ? null : f.label.trim(),
      people: f.kind !== 'closure' && f.people && people > 0 ? Math.trunc(people) : null,
      reason: f.reason.trim() || null,
    };
  }

  private async reload(): Promise<void> {
    try {
      this.periods.set(await firstValueFrom(this.api.list()));
      this.loadError.set(false);
    } catch {
      this.loadError.set(true);
    }
  }
}

function monthOf(day: IsoDate): { year: number; month: number } {
  return { year: Number(day.slice(0, 4)), month: Number(day.slice(5, 7)) - 1 };
}

/** Message of an API error, in French. */
function errorMessage(error: unknown): string {
  const body = error instanceof HttpErrorResponse ? (error.error as ApiErrorBody | null) : null;
  const details = body?.error?.details ?? [];
  if (details.length > 0) {
    return details.join(' ; ');
  }
  return body?.error?.message ?? 'Une erreur est survenue, réessayez plus tard.';
}
