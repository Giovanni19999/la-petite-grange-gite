import { DatePipe, NgTemplateOutlet } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { firstValueFrom } from 'rxjs';

import { ConfirmDialog } from '../../../shared/confirm/confirm-dialog';
import { DateRangeField, DateRangeValue } from '../../../shared/dates/date-range-field';
import {
  IsoDate,
  WEEKDAYS_SHORT,
  formatMonth,
  formatShort,
  monthGrid,
  shiftMonth,
  todayInParis,
} from '../../../shared/dates/dates';
import { ApiErrorBody } from '../pages/data-access/admin-pages.models';
import {
  euros,
  formatEuros,
  formatPercent,
  parseEuros,
  parsePercent,
} from './data-access/price-math';
import {
  Channel,
  NightPrices,
  PriceBook,
  PriceChangeEntry,
  PricedRange,
  PricesApi,
  RangeRequest,
} from './data-access/prices.api';
import { PriceRows, PriceTable, pricesFrom, rowsFrom } from './ui/price-table';

interface RangeDraft {
  range: DateRangeValue;
  label: string;
  minNights: string;
  rows: PriceRows;
}

interface RatesDraft {
  commission: string;
  fees: string;
  vat: boolean;
}

interface Pending {
  title: string;
  description: string;
  action: string;
  destructive: boolean;
  run: () => Promise<unknown>;
  /** Where to show an API refusal. */
  errorKey: string;
}

/** "Tarifs" (F006): prices per channel for the default and each range, validated by hand. */
@Component({
  selector: 'app-prices-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    DatePipe,
    NgTemplateOutlet,
    RouterLink,
    HlmButtonImports,
    ConfirmDialog,
    DateRangeField,
    PriceTable,
  ],
  template: `
    <section class="mx-auto flex max-w-5xl flex-col gap-8 px-4 py-8">
      <div>
        <a routerLink="/admin" class="text-sm text-muted-foreground hover:underline">
          ← Espace administrateur
        </a>
        <h1 class="text-2xl font-semibold">Tarifs</h1>
        <p class="mt-1 text-sm text-muted-foreground">
          Pour chaque canal : <strong>touché</strong> (ce que vous gardez) et
          <strong>affiché au client</strong> (ce que paie le voyageur, à saisir sur la plateforme).
          Saisir l'un calcule l'autre ; les deux restent modifiables. Rien n'est enregistré avant «
          Valider ». Sans channel manager, les prix Airbnb et Booking sont à recopier sur les
          plateformes.
        </p>
      </div>

      @if (loadError()) {
        <p class="text-destructive" role="alert">Impossible de charger les tarifs.</p>
      }

      @if (book(); as b) {
        @if (b.incomplete.length > 0) {
          <p
            class="rounded-md bg-amber-50 p-3 text-sm text-amber-900"
            role="alert"
            data-testid="incomplete"
          >
            Prix incomplet : {{ channelNames(b.incomplete) }} n'{{
              b.incomplete.length > 1 ? 'ont' : 'a'
            }}
            pas de prix par défaut. Ce canal ne peut pas être réservé les nuits sans prix de plage.
          </p>
        }

        <!-- Default prices -->
        <section
          class="flex flex-col gap-3 rounded-lg border p-4"
          aria-labelledby="defaults-title"
          data-testid="defaults"
        >
          <div class="flex flex-wrap items-center justify-between gap-2">
            <h2 id="defaults-title" class="font-semibold">Prix par défaut (par nuit)</h2>
            @if (defaultsDirty()) {
              <span class="text-xs text-amber-900" data-testid="unsaved"
                >Modifications non enregistrées</span
              >
            }
          </div>
          <p class="text-sm text-muted-foreground">Appliqués à toutes les nuits sans période.</p>
          <app-price-table
            caption="Prix par défaut"
            [channels]="b.channels"
            [rows]="defaultRows()"
            [alertPoints]="b.settings.commission_alert_points"
            (rowsChange)="defaultRows.set($event)"
          />
          <ng-container
            *ngTemplateOutlet="stays; context: { $implicit: b.default_typical_stays }"
          />
          @if (errors()['defaults']; as message) {
            <p class="text-sm text-destructive" role="alert">{{ message }}</p>
          }
          <div class="flex gap-2">
            <button
              hlmBtn
              type="button"
              [disabled]="busy() || !defaultsDirty()"
              (click)="askDefaults()"
            >
              Valider
            </button>
            @if (defaultsDirty()) {
              <button hlmBtn variant="outline" type="button" (click)="resetDefaults()">
                Annuler
              </button>
            }
          </div>
        </section>

        <!-- Ranges -->
        <section class="flex flex-col gap-3" aria-labelledby="ranges-title">
          <div class="flex flex-wrap items-center justify-between gap-2">
            <h2 id="ranges-title" class="font-semibold">Périodes</h2>
            <button hlmBtn variant="outline" type="button" (click)="startNewRange()">
              Nouvelle période
            </button>
          </div>
          <p class="text-sm text-muted-foreground">
            Une nouvelle période remplace les périodes qu'elle chevauche sur ses dates. Un canal
            laissé vide utilise son prix par défaut.
          </p>

          @if (newRange(); as draft) {
            <ng-container
              *ngTemplateOutlet="
                rangeEditor;
                context: { $implicit: draft, key: 'new', range: null }
              "
            />
          }
          @for (range of b.ranges; track range.id) {
            <ng-container
              *ngTemplateOutlet="
                rangeEditor;
                context: { $implicit: rangeDraft(range), key: 'range-' + range.id, range: range }
              "
            />
          } @empty {
            @if (!newRange()) {
              <p class="text-sm text-muted-foreground">
                Aucune période : les prix par défaut s'appliquent.
              </p>
            }
          }
        </section>

        <!-- Calendar -->
        <section class="flex flex-col gap-2 rounded-lg border p-4" aria-labelledby="calendar-title">
          <div class="flex items-center justify-between">
            <button
              type="button"
              class="rounded px-2 py-1 hover:bg-muted"
              aria-label="Mois précédent"
              (click)="moveCalendar(-1)"
            >
              ‹
            </button>
            <h2 id="calendar-title" class="font-semibold capitalize">
              Prix du site · {{ calendarTitle() }}
            </h2>
            <button
              type="button"
              class="rounded px-2 py-1 hover:bg-muted"
              aria-label="Mois suivant"
              (click)="moveCalendar(1)"
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
          <div class="grid grid-cols-7 gap-1">
            @for (day of calendarDays(); track $index) {
              @if (day) {
                @let n = nightOf(day);
                <div
                  class="flex aspect-square flex-col items-center justify-center rounded border text-xs"
                  [class.bg-primary/10]="inRange(day)"
                  [attr.aria-label]="nightLabel(day, n)"
                  [attr.data-date]="day"
                >
                  <span class="text-muted-foreground">{{ day.slice(8) }}</span>
                  <span class="font-medium" data-testid="site-price">{{
                    n?.prices['direct'] ? euros(n!.prices['direct']!.listed) : '—'
                  }}</span>
                </div>
              } @else {
                <span></span>
              }
            }
          </div>
          <p class="text-xs text-muted-foreground">En couleur : nuits couvertes par une période.</p>
        </section>

        <!-- Settings and platform rates -->
        <section class="grid gap-4 md:grid-cols-2">
          <form
            class="flex flex-col gap-3 rounded-lg border p-4"
            aria-label="Réglages"
            (submit)="$event.preventDefault(); askSettings()"
          >
            <h2 class="font-semibold">Réglages</h2>
            <label class="flex flex-col gap-1 text-sm">
              Ménage (par séjour, tous canaux)
              <span class="flex items-center gap-1">
                <input
                  #cleaning
                  class="h-9 w-28 rounded-md border border-input px-2 text-right"
                  inputmode="decimal"
                  [value]="settingsDraft().cleaning"
                  (input)="patchSettings({ cleaning: cleaning.value })"
                />
                €
              </span>
            </label>
            <label class="flex flex-col gap-1 text-sm">
              Nuits minimum par défaut
              <input
                #minNights
                class="h-9 w-20 rounded-md border border-input px-2 text-right"
                type="number"
                min="1"
                max="30"
                [value]="settingsDraft().minNights"
                (input)="patchSettings({ minNights: minNights.value })"
              />
            </label>
            <label class="flex flex-col gap-1 text-sm">
              Alerte si la commission s'écarte de plus de (points)
              <input
                #alert
                class="h-9 w-20 rounded-md border border-input px-2 text-right"
                type="number"
                min="0"
                max="100"
                [value]="settingsDraft().alert"
                (input)="patchSettings({ alert: alert.value })"
              />
            </label>
            @if (errors()['settings']; as message) {
              <p class="text-sm text-destructive" role="alert">{{ message }}</p>
            }
            <button hlmBtn type="submit" class="self-start" [disabled]="busy()">Valider</button>
          </form>

          <form
            class="flex flex-col gap-3 rounded-lg border p-4"
            aria-label="Taux des plateformes"
            (submit)="$event.preventDefault(); askRates()"
          >
            <h2 class="font-semibold">Taux des plateformes</h2>
            <p class="text-xs text-muted-foreground">
              Servent seulement à calculer le prix associé pendant la saisie ; changer un taux ne
              modifie aucun prix enregistré.
            </p>
            @for (channel of platformChannels(); track channel.code) {
              @let r = ratesDraft()[channel.code];
              <fieldset
                class="flex flex-wrap items-end gap-2 text-sm"
                [attr.data-channel]="channel.code"
              >
                <legend class="mb-1 font-medium">{{ channel.name }}</legend>
                <label class="flex flex-col gap-1"
                  >Commission %
                  <input
                    #commission
                    class="h-9 w-20 rounded-md border border-input px-2 text-right"
                    inputmode="decimal"
                    [value]="r.commission"
                    (input)="patchRates(channel.code, { commission: commission.value })"
                  />
                </label>
                <label class="flex flex-col gap-1"
                  >Frais de paiement %
                  <input
                    #fees
                    class="h-9 w-20 rounded-md border border-input px-2 text-right"
                    inputmode="decimal"
                    [value]="r.fees"
                    (input)="patchRates(channel.code, { fees: fees.value })"
                  />
                </label>
                <label class="flex items-center gap-2 pb-2">
                  <input
                    #vat
                    type="checkbox"
                    class="size-4 accent-primary"
                    [checked]="r.vat"
                    (change)="patchRates(channel.code, { vat: vat.checked })"
                  />
                  TVA sur la commission
                </label>
              </fieldset>
            }
            @if (errors()['rates']; as message) {
              <p class="text-sm text-destructive" role="alert">{{ message }}</p>
            }
            <button hlmBtn type="submit" class="self-start" [disabled]="busy()">Valider</button>
          </form>
        </section>

        <!-- Tools -->
        <section class="flex flex-col gap-3 rounded-lg border p-4" aria-labelledby="tools-title">
          <h2 id="tools-title" class="font-semibold">Outils</h2>
          <div class="flex flex-wrap items-end gap-2 text-sm">
            <label class="flex flex-col gap-1"
              >Recopier les périodes de l'année
              <input
                #year
                class="h-9 w-24 rounded-md border border-input px-2"
                type="number"
                [value]="currentYear"
              />
            </label>
            <button hlmBtn variant="outline" type="button" (click)="askCopyYear(+year.value)">
              sur l'année suivante
            </button>
          </div>
          <div class="flex flex-wrap items-end gap-2 text-sm">
            <label class="flex flex-col gap-1"
              >Copier les prix de
              <select #from class="h-9 rounded-md border border-input px-2">
                @for (channel of b.channels; track channel.code) {
                  <option [value]="channel.code">{{ channel.name }}</option>
                }
              </select>
            </label>
            <label class="flex flex-col gap-1"
              >vers
              <select #to class="h-9 rounded-md border border-input px-2">
                @for (channel of b.channels; track channel.code) {
                  <option [value]="channel.code" [selected]="$index === 1">
                    {{ channel.name }}
                  </option>
                }
              </select>
            </label>
            <button
              hlmBtn
              variant="outline"
              type="button"
              (click)="askCopyChannel(from.value, to.value)"
            >
              Copier
            </button>
          </div>
          @if (errors()['tools']; as message) {
            <p class="text-sm text-destructive" role="alert">{{ message }}</p>
          }
        </section>

        <details class="rounded-lg border p-3" (toggle)="loadHistory()">
          <summary class="cursor-pointer font-semibold">Historique des modifications</summary>
          <ul class="mt-2 flex flex-col gap-1 text-sm" aria-label="Historique des prix">
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

    <ng-template #stays let-stays>
      @if (book(); as b) {
        <dl
          class="grid gap-x-4 gap-y-1 text-xs text-muted-foreground sm:grid-cols-2"
          aria-label="Séjours types"
        >
          @for (nights of ['2', '7']; track nights) {
            <div>
              <dt class="font-medium">Séjour de {{ nights }} nuits, ménage compris</dt>
              <dd>
                @for (channel of b.channels; track channel.code) {
                  @let total = stays[nights]?.[channel.code];
                  <span class="mr-3 inline-block">
                    {{ channel.name }} :
                    {{
                      total
                        ? euros(total[0]) + ' affiché, ' + euros(total[1]) + ' touché'
                        : 'prix incomplet'
                    }}
                  </span>
                }
              </dd>
            </div>
          }
        </dl>
      }
    </ng-template>

    <ng-template #rangeEditor let-draft let-key="key" let-range="range">
      @if (book(); as b) {
        <article
          class="flex flex-col gap-3 rounded-lg border p-4"
          [attr.data-range]="range?.id ?? 'new'"
        >
          <div class="flex flex-wrap items-center justify-between gap-2">
            <h3 class="font-medium">
              {{
                range
                  ? (range.label ?? 'Période') +
                    ' · du ' +
                    short(range.start) +
                    ' au ' +
                    short(range.end)
                  : 'Nouvelle période'
              }}
            </h3>
            @if (isDirty(key, range)) {
              <span class="text-xs text-amber-900">Modifications non enregistrées</span>
            }
          </div>
          <div class="grid gap-3 md:grid-cols-[1fr_16rem]">
            <app-date-range-field
              legend="Dates"
              startLabel="Du (première nuit)"
              endLabel="Au (lendemain de la dernière nuit)"
              [value]="draft.range"
              (valueChange)="patchRange(key, range, { range: $event })"
            />
            <div class="flex flex-col gap-3 text-sm">
              <label class="flex flex-col gap-1"
                >Nom (facultatif)
                <input
                  #label
                  class="h-9 rounded-md border border-input px-2"
                  maxlength="60"
                  placeholder="Ex. : Été"
                  [value]="draft.label"
                  (input)="patchRange(key, range, { label: label.value })"
                />
              </label>
              <label class="flex flex-col gap-1"
                >Nuits minimum
                <input
                  #min
                  class="h-9 w-20 rounded-md border border-input px-2 text-right"
                  type="number"
                  min="1"
                  max="30"
                  [value]="draft.minNights"
                  (input)="patchRange(key, range, { minNights: min.value })"
                />
              </label>
            </div>
          </div>
          <app-price-table
            [caption]="'Prix de la période'"
            [channels]="b.channels"
            [rows]="draft.rows"
            [alertPoints]="b.settings.commission_alert_points"
            [fallback]="b.defaults"
            (rowsChange)="patchRange(key, range, { rows: $event })"
          />
          @if (range) {
            <ng-container *ngTemplateOutlet="stays; context: { $implicit: range.typical_stays }" />
          }
          @if (errors()[key]; as message) {
            <p class="text-sm text-destructive" role="alert">{{ message }}</p>
          }
          <div class="flex flex-wrap gap-2">
            <button
              hlmBtn
              type="button"
              [disabled]="busy() || (range && !isDirty(key, range))"
              (click)="askRange(key, range, draft)"
            >
              Valider
            </button>
            @if (!range) {
              <button hlmBtn variant="outline" type="button" (click)="newRange.set(null)">
                Annuler
              </button>
            } @else {
              @if (isDirty(key, range)) {
                <button hlmBtn variant="outline" type="button" (click)="resetRange(range)">
                  Annuler
                </button>
              }
              <button
                hlmBtn
                variant="ghost"
                type="button"
                class="ml-auto text-destructive"
                (click)="askDeleteRange(range)"
              >
                Supprimer
              </button>
            }
          </div>
        </article>
      }
    </ng-template>

    <app-confirm-dialog
      [open]="pending() !== null"
      [title]="pending()?.title ?? ''"
      [description]="pending()?.description ?? ''"
      [confirmLabel]="pending()?.action ?? 'Confirmer'"
      [destructive]="pending()?.destructive ?? false"
      (confirmed)="runPending()"
      (cancelled)="pending.set(null)"
    />
  `,
})
export class PricesPage {
  private readonly api = inject(PricesApi);

  protected readonly weekdays = WEEKDAYS_SHORT;
  protected readonly euros = euros;
  protected readonly short = formatShort;
  protected readonly currentYear = Number(todayInParis().slice(0, 4));

  protected readonly book = signal<PriceBook | null>(null);
  protected readonly loadError = signal(false);
  protected readonly busy = signal(false);
  protected readonly errors = signal<Record<string, string | null>>({});
  protected readonly pending = signal<Pending | null>(null);
  protected readonly history = signal<PriceChangeEntry[]>([]);

  protected readonly defaultRows = signal<PriceRows>({});
  protected readonly newRange = signal<RangeDraft | null>(null);
  private readonly rangeDrafts = signal<Record<number, RangeDraft>>({});
  protected readonly settingsDraft = signal({ cleaning: '', minNights: '', alert: '' });
  protected readonly ratesDraft = signal<Record<string, RatesDraft>>({});

  private readonly month = signal(monthOf(todayInParis()));
  private readonly monthNights = signal<NightPrices[]>([]);
  protected readonly calendarDays = computed(() =>
    monthGrid(this.month().year, this.month().month),
  );
  protected readonly calendarTitle = computed(() =>
    formatMonth(this.month().year, this.month().month),
  );

  protected readonly platformChannels = computed(() =>
    (this.book()?.channels ?? []).filter((channel) => channel.code !== 'direct'),
  );
  protected readonly defaultsDirty = computed(() => {
    const book = this.book();
    return (
      book !== null &&
      JSON.stringify(this.defaultRows()) !== JSON.stringify(rowsFrom(book.channels, book.defaults))
    );
  });

  constructor() {
    void this.reload();
  }

  protected channelNames(codes: string[]): string {
    const channels = this.book()?.channels ?? [];
    return codes
      .map((code) => channels.find((channel) => channel.code === code)?.name ?? code)
      .join(', ');
  }

  // ----- Default prices -----

  protected resetDefaults(): void {
    const book = this.book();
    if (book) {
      this.defaultRows.set(rowsFrom(book.channels, book.defaults));
    }
  }

  protected askDefaults(): void {
    const book = this.book();
    if (!book) {
      return;
    }
    const { prices, error } = pricesFrom(book.channels, this.defaultRows());
    this.setError('defaults', error);
    if (error) {
      return;
    }
    this.ask({
      title: 'Valider les prix par défaut ?',
      description: this.describePrices(book.channels, prices),
      action: 'Valider',
      destructive: false,
      errorKey: 'defaults',
      run: () => firstValueFrom(this.api.saveDefaults(prices)),
    });
  }

  // ----- Ranges -----

  protected startNewRange(): void {
    const book = this.book();
    this.newRange.set({
      range: { start: null, end: null },
      label: '',
      minNights: String(book?.settings.default_min_nights ?? 2),
      rows: rowsFrom(book?.channels ?? [], {}),
    });
  }

  protected rangeDraft(range: PricedRange): RangeDraft {
    return this.rangeDrafts()[range.id] ?? this.savedDraft(range);
  }

  protected isDirty(key: string, range: PricedRange | null): boolean {
    if (!range) {
      return key === 'new' && this.newRange() !== null;
    }
    const draft = this.rangeDrafts()[range.id];
    return draft !== undefined && JSON.stringify(draft) !== JSON.stringify(this.savedDraft(range));
  }

  protected patchRange(key: string, range: PricedRange | null, changes: Partial<RangeDraft>): void {
    if (!range) {
      this.newRange.update((draft) => (draft ? { ...draft, ...changes } : draft));
      return;
    }
    this.rangeDrafts.update((drafts) => ({
      ...drafts,
      [range.id]: { ...this.rangeDraft(range), ...changes },
    }));
    this.setError(key, null);
  }

  protected resetRange(range: PricedRange): void {
    this.rangeDrafts.update((drafts) => {
      const next = { ...drafts };
      delete next[range.id];
      return next;
    });
  }

  protected askRange(key: string, range: PricedRange | null, draft: RangeDraft): void {
    const book = this.book();
    if (!book) {
      return;
    }
    const { start, end } = draft.range;
    if (!start || !end || end <= start) {
      this.setError(key, 'Choisissez la première nuit et le lendemain de la dernière nuit.');
      return;
    }
    const minNights = Number(draft.minNights);
    if (!Number.isInteger(minNights) || minNights < 1 || minNights > 30) {
      this.setError(key, 'Nuits minimum : de 1 à 30.');
      return;
    }
    const { prices, error } = pricesFrom(book.channels, draft.rows);
    if (error) {
      this.setError(key, error);
      return;
    }
    this.setError(key, null);
    const request: RangeRequest = {
      start,
      end,
      label: draft.label.trim() || null,
      min_nights: minNights,
      prices: Object.fromEntries(
        Object.entries(prices).filter(
          (entry): entry is [string, NonNullable<(typeof entry)[1]>] => entry[1] !== null,
        ),
      ),
    };
    this.ask({
      title: range ? 'Valider cette période ?' : 'Créer cette période ?',
      description: `Du ${formatShort(start)} au ${formatShort(end)}, ${minNights} nuit(s) minimum. ${this.describePrices(book.channels, prices, true)} Les périodes chevauchées sont remplacées sur ces dates.`,
      action: 'Valider',
      destructive: false,
      errorKey: key,
      run: async () => {
        await firstValueFrom(
          range ? this.api.updateRange(range.id, request) : this.api.createRange(request),
        );
        if (!range) {
          this.newRange.set(null);
        }
      },
    });
  }

  protected askDeleteRange(range: PricedRange): void {
    this.ask({
      title: 'Supprimer cette période ?',
      description: `Du ${formatShort(range.start)} au ${formatShort(range.end)} : ces nuits reprennent les prix par défaut.`,
      action: 'Supprimer',
      destructive: true,
      errorKey: `range-${range.id}`,
      run: () => firstValueFrom(this.api.deleteRange(range.id)),
    });
  }

  // ----- Settings, rates, tools -----

  protected patchSettings(
    changes: Partial<{ cleaning: string; minNights: string; alert: string }>,
  ): void {
    this.settingsDraft.update((draft) => ({ ...draft, ...changes }));
  }

  protected askSettings(): void {
    const draft = this.settingsDraft();
    const cleaning = parseEuros(draft.cleaning || '0');
    const minNights = Number(draft.minNights);
    const alert = Number(draft.alert);
    if (
      cleaning === null ||
      !Number.isInteger(minNights) ||
      minNights < 1 ||
      minNights > 30 ||
      !Number.isInteger(alert) ||
      alert < 0 ||
      alert > 100
    ) {
      this.setError(
        'settings',
        'Vérifiez le ménage (en euros), les nuits minimum (1 à 30) et le seuil (0 à 100).',
      );
      return;
    }
    this.setError('settings', null);
    this.ask({
      title: 'Valider les réglages ?',
      description: `Ménage ${euros(cleaning)} par séjour, ${minNights} nuit(s) minimum par défaut.`,
      action: 'Valider',
      destructive: false,
      errorKey: 'settings',
      run: () =>
        firstValueFrom(
          this.api.saveSettings({
            cleaning_fee: cleaning,
            default_min_nights: minNights,
            commission_alert_points: alert,
          }),
        ),
    });
  }

  protected patchRates(code: string, changes: Partial<RatesDraft>): void {
    this.ratesDraft.update((drafts) => ({ ...drafts, [code]: { ...drafts[code], ...changes } }));
  }

  protected askRates(): void {
    const parsed = this.platformChannels().map((channel) => {
      const draft = this.ratesDraft()[channel.code];
      return {
        channel,
        commission: parsePercent(draft.commission),
        fees: parsePercent(draft.fees),
        vat: draft.vat,
      };
    });
    if (parsed.some((rate) => rate.commission === null || rate.fees === null)) {
      this.setError('rates', 'Les taux sont des pourcentages entre 0 et 100.');
      return;
    }
    this.setError('rates', null);
    this.ask({
      title: 'Valider les taux des plateformes ?',
      description:
        'Les prix déjà enregistrés ne changent pas ; les prochains calculs utiliseront ces taux.',
      action: 'Valider',
      destructive: false,
      errorKey: 'rates',
      run: async () => {
        for (const rate of parsed) {
          await firstValueFrom(
            this.api.saveRates(rate.channel.code, {
              commission_bp: rate.commission!,
              payment_fees_bp: rate.fees!,
              vat_on_commission: rate.vat,
            }),
          );
        }
      },
    });
  }

  protected askCopyYear(year: number): void {
    if (!Number.isInteger(year)) {
      return;
    }
    this.ask({
      title: `Recopier les périodes de ${year} sur ${year + 1} ?`,
      description: `Chaque période de ${year} est recopiée 52 semaines plus tard (mêmes jours de la semaine) avec ses prix ; elle remplace ce qui existe déjà sur ces dates.`,
      action: 'Recopier',
      destructive: false,
      errorKey: 'tools',
      run: () => firstValueFrom(this.api.copyYear(year)),
    });
  }

  protected askCopyChannel(from: string, to: string): void {
    if (from === to) {
      this.setError('tools', 'Choisissez deux canaux différents.');
      return;
    }
    this.ask({
      title: `Copier les prix de ${this.channelNames([from])} vers ${this.channelNames([to])} ?`,
      description: `Les prix par défaut et de toutes les périodes de ${this.channelNames([to])} sont remplacés par ceux de ${this.channelNames([from])} (modifiables ensuite).`,
      action: 'Copier',
      destructive: false,
      errorKey: 'tools',
      run: () => firstValueFrom(this.api.copyChannel(from, to)),
    });
  }

  // ----- Calendar, history -----

  protected moveCalendar(delta: number): void {
    this.month.update(({ year, month }) => shiftMonth(year, month, delta));
    void this.loadMonth();
  }

  protected nightOf(day: IsoDate): NightPrices | null {
    return this.monthNights().find((night) => night.date === day) ?? null;
  }

  protected inRange(day: IsoDate): boolean {
    return (this.book()?.ranges ?? []).some((range) => day >= range.start && day < range.end);
  }

  protected nightLabel(day: IsoDate, night: NightPrices | null): string {
    if (!night) {
      return day;
    }
    const channels = this.book()?.channels ?? [];
    const prices = channels
      .map((channel) => {
        const price = night.prices[channel.code];
        return `${channel.name} ${price ? euros(price.listed) + ' affiché, ' + euros(price.net) + ' touché' : 'prix incomplet'}`;
      })
      .join(' ; ');
    return `${formatShort(day)} : ${prices} ; ${night.min_nights} nuit(s) minimum`;
  }

  protected async loadHistory(): Promise<void> {
    try {
      this.history.set(await firstValueFrom(this.api.history()));
    } catch {
      this.history.set([]);
    }
  }

  protected async runPending(): Promise<void> {
    const pending = this.pending();
    this.pending.set(null);
    if (!pending) {
      return;
    }
    this.busy.set(true);
    try {
      await pending.run();
      this.setError(pending.errorKey, null);
      await this.reload();
      if (this.history().length > 0) {
        await this.loadHistory();
      }
    } catch (error) {
      this.setError(pending.errorKey, errorMessage(error));
    } finally {
      this.busy.set(false);
    }
  }

  private ask(pending: Pending): void {
    this.pending.set(pending);
  }

  private setError(key: string, message: string | null): void {
    this.errors.update((errors) => ({ ...errors, [key]: message }));
  }

  private describePrices(
    channels: Channel[],
    prices: Record<string, { listed: number; net: number } | null>,
    inRange = false,
  ): string {
    return (
      channels
        .map((channel) => {
          const price = prices[channel.code];
          return price
            ? `${channel.name} : ${euros(price.net)} touché, ${euros(price.listed)} affiché`
            : `${channel.name} : ${inRange ? 'prix par défaut' : 'sans prix'}`;
        })
        .join(' ; ') + '.'
    );
  }

  private savedDraft(range: PricedRange): RangeDraft {
    return {
      range: { start: range.start, end: range.end },
      label: range.label ?? '',
      minNights: String(range.min_nights),
      rows: rowsFrom(this.book()?.channels ?? [], range.prices),
    };
  }

  private async reload(): Promise<void> {
    try {
      const book = await firstValueFrom(this.api.book());
      this.book.set(book);
      this.defaultRows.set(rowsFrom(book.channels, book.defaults));
      this.rangeDrafts.set({});
      this.settingsDraft.set({
        cleaning: formatEuros(book.settings.cleaning_fee),
        minNights: String(book.settings.default_min_nights),
        alert: String(book.settings.commission_alert_points),
      });
      this.ratesDraft.set(
        Object.fromEntries(
          book.channels.map((channel) => [
            channel.code,
            {
              commission: formatPercent(channel.commission_bp),
              fees: formatPercent(channel.payment_fees_bp),
              vat: channel.vat_on_commission,
            },
          ]),
        ),
      );
      this.loadError.set(false);
      await this.loadMonth();
    } catch {
      this.loadError.set(true);
    }
  }

  private async loadMonth(): Promise<void> {
    const { year, month } = this.month();
    const from = `${year}-${String(month + 1).padStart(2, '0')}-01`;
    const next = shiftMonth(year, month, 1);
    const to = `${next.year}-${String(next.month + 1).padStart(2, '0')}-01`;
    try {
      this.monthNights.set(await firstValueFrom(this.api.nights(from, to)));
    } catch {
      this.monthNights.set([]);
    }
  }
}

function monthOf(day: IsoDate): { year: number; month: number } {
  return { year: Number(day.slice(0, 4)), month: Number(day.slice(5, 7)) - 1 };
}

function errorMessage(error: unknown): string {
  const body = error instanceof HttpErrorResponse ? (error.error as ApiErrorBody | null) : null;
  const details = body?.error?.details ?? [];
  if (details.length > 0) {
    return details.join(' ; ');
  }
  return body?.error?.message ?? 'Une erreur est survenue, réessayez plus tard.';
}
