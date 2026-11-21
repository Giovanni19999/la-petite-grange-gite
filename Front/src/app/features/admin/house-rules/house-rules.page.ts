import { DatePipe } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { firstValueFrom } from 'rxjs';

import { Choice, ChoiceGroup } from '../../../shared/choice/choice-group';
import { ApiErrorBody } from '../pages/data-access/admin-pages.models';
import {
  formatEuros,
  formatPercent,
  parseEuros,
  parsePercent,
} from '../prices/data-access/price-math';
import { HouseRules, HouseRulesApi, RuleChangeEntry } from './data-access/house-rules.api';

/** The form as typed (text, so that "1,70" can be typed). */
interface RulesDraft {
  capacity: string;
  minDelay: string;
  arrivals: number[];
  departures: number[];
  depositMode: 'percentage' | 'fixed';
  depositPercent: string;
  depositAmount: string;
  balanceDays: string;
  taxPerNight: string;
  taxAdditional: string;
  exemptUnder: string;
  horizonMonths: string;
  horizonUntil: string;
}

const WEEKDAYS: readonly { number: number; short: string; name: string }[] = [
  { number: 1, short: 'Lun', name: 'lundi' },
  { number: 2, short: 'Mar', name: 'mardi' },
  { number: 3, short: 'Mer', name: 'mercredi' },
  { number: 4, short: 'Jeu', name: 'jeudi' },
  { number: 5, short: 'Ven', name: 'vendredi' },
  { number: 6, short: 'Sam', name: 'samedi' },
  { number: 7, short: 'Dim', name: 'dimanche' },
];

const DEPOSIT_MODES: readonly Choice<'percentage' | 'fixed'>[] = [
  { value: 'percentage', label: 'Pourcentage' },
  { value: 'fixed', label: 'Montant fixe' },
];

function draftOf(rules: HouseRules): RulesDraft {
  return {
    capacity: String(rules.capacity),
    minDelay: String(rules.min_delay_days),
    arrivals: [...rules.arrival_weekdays],
    departures: [...rules.departure_weekdays],
    depositMode: rules.deposit.mode,
    depositPercent: rules.deposit.mode === 'percentage' ? String(rules.deposit.percent) : '50',
    depositAmount: rules.deposit.mode === 'fixed' ? formatEuros(rules.deposit.amount) : '',
    balanceDays: String(rules.balance_days),
    taxPerNight: formatEuros(rules.tourist_tax.per_adult_night),
    taxAdditional: formatPercent(rules.tourist_tax.additional_bp),
    exemptUnder: String(rules.tourist_tax.exempt_under),
    horizonMonths: String(rules.horizon.months),
    horizonUntil: rules.horizon.until ?? '',
  };
}

/** Whole number from a field, `null` if it is not one. */
function whole(text: string): number | null {
  return /^\d+$/.test(text.trim()) ? Number(text.trim()) : null;
}

/** "Règles du gîte" (F004): the rules the quote depends on, with their history. */
@Component({
  selector: 'app-house-rules-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [DatePipe, RouterLink, HlmButtonImports, ChoiceGroup],
  template: `
    <section class="mx-auto flex max-w-3xl flex-col gap-6 px-4 py-8">
      <div>
        <a routerLink="/admin" class="text-sm text-muted-foreground hover:underline">
          ← Espace administrateur
        </a>
        <h1 class="text-2xl font-semibold">Règles du gîte</h1>
        <p class="mt-1 text-sm text-muted-foreground">
          Elles s'appliquent aux devis et réservations du site. Les nuits minimum et le ménage se
          règlent dans <a routerLink="/admin/tarifs" class="underline">Tarifs</a>.
        </p>
      </div>

      @if (loadError()) {
        <p class="text-destructive" role="alert">Impossible de charger les règles.</p>
      }

      @if (draft(); as d) {
        <form class="flex flex-col gap-6" (submit)="$event.preventDefault(); save()">
          <fieldset class="flex flex-col gap-3 rounded-lg border p-4">
            <legend class="px-1 font-semibold">Séjour</legend>
            <label class="flex flex-col gap-1 text-sm">
              Capacité (adultes et enfants)
              <input
                #capacity
                class="h-9 w-24 rounded-md border border-input px-2 text-right"
                inputmode="numeric"
                name="capacity"
                [value]="d.capacity"
                (input)="patch({ capacity: capacity.value })"
              />
            </label>
            <label class="flex flex-col gap-1 text-sm">
              Délai minimum avant l'arrivée (jours, 1 = arrivée demain au plus tôt)
              <input
                #minDelay
                class="h-9 w-24 rounded-md border border-input px-2 text-right"
                inputmode="numeric"
                name="minDelay"
                [value]="d.minDelay"
                (input)="patch({ minDelay: minDelay.value })"
              />
            </label>
            @for (kind of weekdayKinds; track kind.key) {
              <fieldset class="flex flex-col gap-1 text-sm" [attr.data-weekdays]="kind.key">
                <legend class="mb-1">{{ kind.label }}</legend>
                <div class="flex flex-wrap gap-1">
                  @for (day of weekdays; track day.number) {
                    <label
                      class="cursor-pointer rounded-md border px-2 py-1 has-checked:border-primary has-checked:bg-primary/10 has-focus-visible:ring-2 has-focus-visible:ring-ring"
                      [title]="day.name"
                    >
                      <input
                        class="sr-only"
                        type="checkbox"
                        [checked]="d[kind.key].includes(day.number)"
                        [attr.aria-label]="kind.label + ' : ' + day.name"
                        (change)="toggle(kind.key, day.number)"
                      />
                      {{ day.short }}
                    </label>
                  }
                </div>
              </fieldset>
            }
          </fieldset>

          <fieldset class="flex flex-col gap-3 rounded-lg border p-4">
            <legend class="px-1 font-semibold">Paiement</legend>
            <app-choice-group
              legend="Acompte à la réservation"
              [choices]="depositModes"
              [value]="d.depositMode"
              (changed)="patch({ depositMode: $event })"
            />
            @if (d.depositMode === 'percentage') {
              <label class="flex items-center gap-2 text-sm">
                <input
                  #percent
                  class="h-9 w-20 rounded-md border border-input px-2 text-right"
                  inputmode="numeric"
                  name="depositPercent"
                  aria-label="Acompte en pourcentage du séjour"
                  [value]="d.depositPercent"
                  (input)="patch({ depositPercent: percent.value })"
                />
                % du séjour (nuits et ménage)
              </label>
            } @else {
              <label class="flex items-center gap-2 text-sm">
                <input
                  #amount
                  class="h-9 w-24 rounded-md border border-input px-2 text-right"
                  inputmode="decimal"
                  name="depositAmount"
                  aria-label="Montant de l'acompte"
                  [value]="d.depositAmount"
                  (input)="patch({ depositAmount: amount.value })"
                />
                € (au plus le prix du séjour)
              </label>
            }
            <label class="flex flex-col gap-1 text-sm">
              Solde dû (jours avant l'arrivée)
              <input
                #balance
                class="h-9 w-24 rounded-md border border-input px-2 text-right"
                inputmode="numeric"
                name="balanceDays"
                [value]="d.balanceDays"
                (input)="patch({ balanceDays: balance.value })"
              />
            </label>
          </fieldset>

          <fieldset class="flex flex-col gap-3 rounded-lg border p-4">
            <legend class="px-1 font-semibold">Taxe de séjour (payée sur place)</legend>
            <label class="flex flex-col gap-1 text-sm">
              Par adulte et par nuit (tarif voté par la commune)
              <span class="flex items-center gap-1">
                <input
                  #tax
                  class="h-9 w-24 rounded-md border border-input px-2 text-right"
                  inputmode="decimal"
                  name="taxPerNight"
                  [value]="d.taxPerNight"
                  (input)="patch({ taxPerNight: tax.value })"
                />
                €
              </span>
            </label>
            <label class="flex flex-col gap-1 text-sm">
              Taxe additionnelle (départementale)
              <span class="flex items-center gap-1">
                <input
                  #additional
                  class="h-9 w-24 rounded-md border border-input px-2 text-right"
                  inputmode="decimal"
                  name="taxAdditional"
                  [value]="d.taxAdditional"
                  (input)="patch({ taxAdditional: additional.value })"
                />
                %
              </span>
            </label>
            <label class="flex flex-col gap-1 text-sm">
              Exonérés en dessous de (âge)
              <input
                #exempt
                class="h-9 w-24 rounded-md border border-input px-2 text-right"
                inputmode="numeric"
                name="exemptUnder"
                [value]="d.exemptUnder"
                (input)="patch({ exemptUnder: exempt.value })"
              />
            </label>
          </fieldset>

          <fieldset class="flex flex-col gap-3 rounded-lg border p-4">
            <legend class="px-1 font-semibold">Réservable à l'avance</legend>
            <label class="flex flex-col gap-1 text-sm">
              Nombre de mois
              <input
                #months
                class="h-9 w-24 rounded-md border border-input px-2 text-right"
                inputmode="numeric"
                name="horizonMonths"
                [value]="d.horizonMonths"
                (input)="patch({ horizonMonths: months.value })"
              />
            </label>
            <label class="flex flex-col gap-1 text-sm">
              Et au plus tard le (facultatif)
              <input
                #until
                class="h-9 w-44 rounded-md border border-input px-2"
                type="date"
                name="horizonUntil"
                [value]="d.horizonUntil"
                (input)="patch({ horizonUntil: until.value })"
              />
            </label>
          </fieldset>

          @if (errors().length > 0) {
            <ul class="list-inside list-disc text-sm text-destructive" role="alert">
              @for (error of errors(); track error) {
                <li>{{ error }}</li>
              }
            </ul>
          }
          <div class="flex flex-wrap items-center gap-3">
            <button hlmBtn type="submit" [disabled]="busy() || !dirty()">Valider</button>
            @if (dirty()) {
              <button hlmBtn variant="outline" type="button" (click)="reset()">Annuler</button>
              <span class="text-xs text-amber-900" data-testid="unsaved">
                Modifications non enregistrées
              </span>
            } @else if (saved()) {
              <span class="text-sm text-muted-foreground" role="status">Règles enregistrées.</span>
            }
          </div>
        </form>

        <details class="rounded-lg border p-3" (toggle)="loadHistory()">
          <summary class="cursor-pointer font-semibold">Historique des modifications</summary>
          <ul class="mt-2 flex flex-col gap-1 text-sm" aria-label="Historique des règles">
            @for (entry of history(); track $index) {
              <li>
                {{ entry.description }} · {{ entry.author ?? 'compte supprimé' }} ·
                <span class="text-muted-foreground">{{ entry.at | date: 'dd/MM/yyyy HH:mm' }}</span>
              </li>
            } @empty {
              <li class="text-muted-foreground">Aucune modification.</li>
            }
          </ul>
        </details>
      }
    </section>
  `,
})
export class HouseRulesPage {
  private readonly api = inject(HouseRulesApi);

  protected readonly weekdays = WEEKDAYS;
  protected readonly depositModes = DEPOSIT_MODES;
  protected readonly weekdayKinds = [
    { key: 'arrivals', label: "Jours d'arrivée" },
    { key: 'departures', label: 'Jours de départ' },
  ] as const;

  private readonly saved$ = signal<HouseRules | null>(null);
  protected readonly draft = signal<RulesDraft | null>(null);
  protected readonly loadError = signal(false);
  protected readonly busy = signal(false);
  protected readonly saved = signal(false);
  protected readonly errors = signal<string[]>([]);
  protected readonly history = signal<RuleChangeEntry[]>([]);

  protected readonly dirty = computed(() => {
    const saved = this.saved$();
    const draft = this.draft();
    return (
      saved !== null && draft !== null && JSON.stringify(draftOf(saved)) !== JSON.stringify(draft)
    );
  });

  constructor() {
    void this.load();
  }

  protected patch(changes: Partial<RulesDraft>): void {
    this.draft.update((draft) => (draft ? { ...draft, ...changes } : draft));
    this.saved.set(false);
  }

  protected toggle(key: 'arrivals' | 'departures', day: number): void {
    const days = this.draft()?.[key] ?? [];
    const next = days.includes(day) ? days.filter((d) => d !== day) : [...days, day];
    this.patch({ [key]: next.sort((a, b) => a - b) });
  }

  protected reset(): void {
    const saved = this.saved$();
    if (saved) {
      this.draft.set(draftOf(saved));
    }
    this.errors.set([]);
  }

  protected async save(): Promise<void> {
    const draft = this.draft();
    if (!draft) {
      return;
    }
    const { rules, errors } = rulesFrom(draft);
    this.errors.set(errors);
    if (!rules) {
      return;
    }
    this.busy.set(true);
    try {
      await firstValueFrom(this.api.save(rules));
      await this.load();
      this.saved.set(true);
      if (this.history().length > 0) {
        await this.loadHistory();
      }
    } catch (error) {
      this.errors.set(errorMessages(error));
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

  private async load(): Promise<void> {
    try {
      const rules = await firstValueFrom(this.api.rules());
      this.saved$.set(rules);
      this.draft.set(draftOf(rules));
      this.loadError.set(false);
    } catch {
      this.loadError.set(true);
    }
  }
}

/** The form → the rules, or the problems to fix (the API checks the bounds again). */
export function rulesFrom(draft: RulesDraft): { rules: HouseRules | null; errors: string[] } {
  const errors: string[] = [];
  const need = <T>(value: T | null, message: string): T => {
    if (value === null) {
      errors.push(message);
    }
    return value as T;
  };
  const capacity = need(whole(draft.capacity), 'Capacité : un nombre entier de personnes.');
  const minDelay = need(whole(draft.minDelay), 'Délai minimum : un nombre entier de jours.');
  if (draft.arrivals.length === 0) {
    errors.push("Choisissez au moins un jour d'arrivée.");
  }
  if (draft.departures.length === 0) {
    errors.push('Choisissez au moins un jour de départ.');
  }
  const percent =
    draft.depositMode === 'percentage'
      ? need(whole(draft.depositPercent), 'Acompte : un pourcentage entier.')
      : 0;
  const amount =
    draft.depositMode === 'fixed'
      ? need(parseEuros(draft.depositAmount), 'Acompte : un montant en euros.')
      : 0;
  const balanceDays = need(whole(draft.balanceDays), 'Solde : un nombre entier de jours.');
  const taxPerNight = need(
    parseEuros(draft.taxPerNight || '0'),
    'Taxe de séjour : un montant en euros, par exemple 1,70.',
  );
  const additional = need(
    parsePercent(draft.taxAdditional || '0'),
    'Taxe additionnelle : un pourcentage entre 0 et 100.',
  );
  const exemptUnder = need(whole(draft.exemptUnder), "Âge d'exonération : un nombre entier.");
  const months = need(whole(draft.horizonMonths), 'Réservable : un nombre entier de mois.');
  if (errors.length > 0) {
    return { rules: null, errors };
  }
  return {
    errors,
    rules: {
      capacity,
      min_delay_days: minDelay,
      arrival_weekdays: draft.arrivals,
      departure_weekdays: draft.departures,
      deposit:
        draft.depositMode === 'percentage'
          ? { mode: 'percentage', percent }
          : { mode: 'fixed', amount },
      balance_days: balanceDays,
      tourist_tax: {
        per_adult_night: taxPerNight,
        additional_bp: additional,
        exempt_under: exemptUnder,
      },
      horizon: { months, until: draft.horizonUntil || null },
    },
  };
}

function errorMessages(error: unknown): string[] {
  const body = error instanceof HttpErrorResponse ? (error.error as ApiErrorBody | null) : null;
  const details = body?.error?.details ?? [];
  if (details.length > 0) {
    return details;
  }
  return [body?.error?.message ?? 'Une erreur est survenue, réessayez plus tard.'];
}
