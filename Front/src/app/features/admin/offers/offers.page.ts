import { NgTemplateOutlet } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { firstValueFrom } from 'rxjs';

import { Choice, ChoiceGroup } from '../../../shared/choice/choice-group';
import { ConfirmDialog } from '../../../shared/confirm/confirm-dialog';
import { formatShort } from '../../../shared/dates/dates';
import { ApiErrorBody } from '../pages/data-access/admin-pages.models';
import { euros, formatPercent } from '../prices/data-access/price-math';
import {
  EMPTY_OPTION,
  EMPTY_PROMOTION,
  OptionDraft,
  PromotionDraft,
  optionDraft,
  optionFrom,
  promotionDraft,
  promotionFrom,
} from './data-access/offer-drafts';
import {
  OffersApi,
  OptionUnit,
  Promotion,
  StayOption,
  StoredOption,
  StoredPromotion,
} from './data-access/offers.api';

type Editing =
  | { what: 'promotion'; id: number | null; draft: PromotionDraft }
  | { what: 'option'; id: number | null; draft: OptionDraft };

const KINDS: readonly Choice<PromotionDraft['kind']>[] = [
  { value: 'percent', label: 'Pourcentage' },
  { value: 'per_stay', label: '€ par séjour' },
  { value: 'per_night', label: '€ par nuit' },
];

const UNITS: readonly Choice<OptionUnit>[] = [
  { value: 'stay', label: 'Par séjour' },
  { value: 'night', label: 'Par nuit' },
  { value: 'guest', label: 'Par personne' },
  { value: 'guest_night', label: 'Par personne et par nuit' },
];

const UNIT_TEXT: Record<OptionUnit, string> = {
  stay: 'par séjour',
  night: 'par nuit',
  guest: 'par personne',
  guest_night: 'par personne et par nuit',
};

/** "dès 7 nuits", "jusqu'à 3 nuits", "de 3 à 6 nuits" or "toute durée". */
export function describeNights(min: number | null, max: number | null): string {
  if (min !== null && max !== null) {
    return `de ${min} à ${max} nuits`;
  }
  if (min !== null) {
    return `dès ${min} nuits`;
  }
  return max !== null ? `jusqu'à ${max} nuits` : 'toute durée';
}

/** "Promotions et options" (F026): what the direct booking offers by stay length. */
@Component({
  selector: 'app-offers-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [NgTemplateOutlet, RouterLink, HlmButtonImports, ChoiceGroup, ConfirmDialog],
  template: `
    <section class="mx-auto flex max-w-3xl flex-col gap-8 px-4 py-8">
      <div>
        <a routerLink="/admin" class="text-sm text-muted-foreground hover:underline">
          ← Espace administrateur
        </a>
        <h1 class="text-2xl font-semibold">Promotions et options</h1>
        <p class="mt-1 text-sm text-muted-foreground">
          Pour les réservations sur le site seulement. Une seule promotion s'applique : la plus
          avantageuse. Les nuits minimum et maximum sont facultatives et incluses. Les modifications
          apparaissent dans l'historique de
          <a routerLink="/admin/tarifs" class="underline">Tarifs</a>.
        </p>
      </div>

      @if (loadError()) {
        <p class="text-destructive" role="alert">
          Impossible de charger les promotions et options.
        </p>
      }

      <section class="flex flex-col gap-3" aria-labelledby="promotions-title">
        <div class="flex items-center justify-between gap-2">
          <h2 id="promotions-title" class="font-semibold">Promotions</h2>
          <button hlmBtn variant="outline" type="button" (click)="editPromotion(null)">
            Nouvelle promotion
          </button>
        </div>
        <ul class="flex flex-col gap-2" data-testid="promotions">
          @for (promotion of promotions(); track promotion.id) {
            <li
              class="flex flex-wrap items-center justify-between gap-2 rounded-lg border p-3 text-sm"
            >
              <span [class.text-muted-foreground]="!promotion.active">
                <strong>{{ promotion.name.fr }}</strong> · {{ discountText(promotion) }} ·
                {{ nightsText(promotion) }}{{ datesText(promotion) }}
                @if (!promotion.active) {
                  · inactive
                }
              </span>
              <span class="flex gap-2">
                <button
                  hlmBtn
                  size="sm"
                  variant="outline"
                  type="button"
                  (click)="editPromotion(promotion)"
                >
                  Modifier
                </button>
                <button
                  hlmBtn
                  size="sm"
                  variant="ghost"
                  type="button"
                  class="text-destructive"
                  (click)="askDelete('promotion', promotion.id, promotion.name.fr)"
                >
                  Supprimer
                </button>
              </span>
            </li>
          } @empty {
            <li class="text-sm text-muted-foreground">Aucune promotion.</li>
          }
        </ul>
        @if (editing(); as e) {
          @if (e.what === 'promotion') {
            <form
              class="flex flex-col gap-3 rounded-lg border p-4"
              aria-label="Promotion"
              (submit)="$event.preventDefault(); save()"
            >
              <h3 class="font-medium">
                {{ e.id === null ? 'Nouvelle promotion' : 'Modifier la promotion' }}
              </h3>
              <div class="grid gap-3 sm:grid-cols-2">
                <label class="flex flex-col gap-1 text-sm">
                  Nom (français)
                  <input
                    #nameFr
                    class="h-9 rounded-md border border-input px-2"
                    name="nameFr"
                    maxlength="80"
                    [value]="e.draft.nameFr"
                    (input)="patchPromotion({ nameFr: nameFr.value })"
                  />
                </label>
                <label class="flex flex-col gap-1 text-sm">
                  Nom (anglais, facultatif)
                  <input
                    #nameEn
                    class="h-9 rounded-md border border-input px-2"
                    name="nameEn"
                    maxlength="80"
                    [value]="e.draft.nameEn"
                    (input)="patchPromotion({ nameEn: nameEn.value })"
                  />
                </label>
              </div>
              <app-choice-group
                legend="Remise"
                [choices]="kinds"
                [value]="e.draft.kind"
                (changed)="patchPromotion({ kind: $event })"
              />
              <label class="flex items-center gap-2 text-sm">
                <input
                  #value
                  class="h-9 w-24 rounded-md border border-input px-2 text-right"
                  inputmode="decimal"
                  name="value"
                  [attr.aria-label]="'Montant de la remise'"
                  [value]="e.draft.value"
                  (input)="patchPromotion({ value: value.value })"
                />
                {{
                  e.draft.kind === 'percent'
                    ? '% des nuits'
                    : e.draft.kind === 'per_stay'
                      ? '€ par séjour'
                      : '€ par nuit'
                }}
              </label>
              <ng-container
                *ngTemplateOutlet="bounds; context: { $implicit: e.draft, patch: patchPromotionFn }"
              />
              <div class="grid gap-3 sm:grid-cols-2">
                <label class="flex flex-col gap-1 text-sm">
                  Arrivées à partir du (facultatif)
                  <input
                    #from
                    class="h-9 rounded-md border border-input px-2"
                    type="date"
                    name="from"
                    [value]="e.draft.from"
                    (input)="patchPromotion({ from: from.value })"
                  />
                </label>
                <label class="flex flex-col gap-1 text-sm">
                  Jusqu'au (facultatif)
                  <input
                    #until
                    class="h-9 rounded-md border border-input px-2"
                    type="date"
                    name="until"
                    [value]="e.draft.until"
                    (input)="patchPromotion({ until: until.value })"
                  />
                </label>
              </div>
              <ng-container *ngTemplateOutlet="footer; context: { $implicit: e.draft.active }" />
            </form>
          }
        }
      </section>

      <section class="flex flex-col gap-3" aria-labelledby="options-title">
        <div class="flex items-center justify-between gap-2">
          <h2 id="options-title" class="font-semibold">Options</h2>
          <button hlmBtn variant="outline" type="button" (click)="editOption(null)">
            Nouvelle option
          </button>
        </div>
        <ul class="flex flex-col gap-2" data-testid="options">
          @for (option of options(); track option.id) {
            <li
              class="flex flex-wrap items-center justify-between gap-2 rounded-lg border p-3 text-sm"
            >
              <span [class.text-muted-foreground]="!option.active">
                <strong>{{ option.name.fr }}</strong> · {{ priceText(option) }} ·
                {{ nightsText(option) }}
                @if (!option.active) {
                  · inactive
                }
              </span>
              <span class="flex gap-2">
                <button
                  hlmBtn
                  size="sm"
                  variant="outline"
                  type="button"
                  (click)="editOption(option)"
                >
                  Modifier
                </button>
                <button
                  hlmBtn
                  size="sm"
                  variant="ghost"
                  type="button"
                  class="text-destructive"
                  (click)="askDelete('option', option.id, option.name.fr)"
                >
                  Supprimer
                </button>
              </span>
            </li>
          } @empty {
            <li class="text-sm text-muted-foreground">Aucune option.</li>
          }
        </ul>
        @if (editing(); as e) {
          @if (e.what === 'option') {
            <form
              class="flex flex-col gap-3 rounded-lg border p-4"
              aria-label="Option"
              (submit)="$event.preventDefault(); save()"
            >
              <h3 class="font-medium">
                {{ e.id === null ? 'Nouvelle option' : "Modifier l'option" }}
              </h3>
              <div class="grid gap-3 sm:grid-cols-2">
                <label class="flex flex-col gap-1 text-sm">
                  Nom (français)
                  <input
                    #nameFr
                    class="h-9 rounded-md border border-input px-2"
                    name="nameFr"
                    maxlength="80"
                    [value]="e.draft.nameFr"
                    (input)="patchOption({ nameFr: nameFr.value })"
                  />
                </label>
                <label class="flex flex-col gap-1 text-sm">
                  Nom (anglais, facultatif)
                  <input
                    #nameEn
                    class="h-9 rounded-md border border-input px-2"
                    name="nameEn"
                    maxlength="80"
                    [value]="e.draft.nameEn"
                    (input)="patchOption({ nameEn: nameEn.value })"
                  />
                </label>
                <label class="flex flex-col gap-1 text-sm">
                  Description (français, facultatif)
                  <textarea
                    #descFr
                    class="min-h-16 rounded-md border border-input p-2"
                    name="descriptionFr"
                    maxlength="300"
                    [value]="e.draft.descriptionFr"
                    (input)="patchOption({ descriptionFr: descFr.value })"
                  ></textarea>
                </label>
                <label class="flex flex-col gap-1 text-sm">
                  Description (anglais, facultatif)
                  <textarea
                    #descEn
                    class="min-h-16 rounded-md border border-input p-2"
                    name="descriptionEn"
                    maxlength="300"
                    [value]="e.draft.descriptionEn"
                    (input)="patchOption({ descriptionEn: descEn.value })"
                  ></textarea>
                </label>
              </div>
              <app-choice-group
                legend="Prix"
                [choices]="units"
                [value]="e.draft.unit"
                (changed)="patchOption({ unit: $event })"
              />
              <label class="flex items-center gap-2 text-sm">
                <input
                  #price
                  class="h-9 w-24 rounded-md border border-input px-2 text-right"
                  inputmode="decimal"
                  name="price"
                  aria-label="Prix de l'option"
                  [value]="e.draft.price"
                  (input)="patchOption({ price: price.value })"
                />
                € (0 : offert)
              </label>
              <ng-container
                *ngTemplateOutlet="bounds; context: { $implicit: e.draft, patch: patchOptionFn }"
              />
              <label class="flex flex-col gap-1 text-sm">
                Ordre d'affichage
                <input
                  #position
                  class="h-9 w-20 rounded-md border border-input px-2 text-right"
                  inputmode="numeric"
                  name="position"
                  [value]="e.draft.position"
                  (input)="patchOption({ position: position.value })"
                />
              </label>
              <ng-container *ngTemplateOutlet="footer; context: { $implicit: e.draft.active }" />
            </form>
          }
        }
      </section>
    </section>

    <ng-template #bounds let-draft let-patch="patch">
      <div class="grid gap-3 sm:grid-cols-2">
        <label class="flex flex-col gap-1 text-sm">
          Nuits minimum (facultatif)
          <input
            #min
            class="h-9 w-24 rounded-md border border-input px-2 text-right"
            inputmode="numeric"
            name="min"
            [value]="draft.min"
            (input)="patch({ min: min.value })"
          />
        </label>
        <label class="flex flex-col gap-1 text-sm">
          Nuits maximum (facultatif)
          <input
            #max
            class="h-9 w-24 rounded-md border border-input px-2 text-right"
            inputmode="numeric"
            name="max"
            [value]="draft.max"
            (input)="patch({ max: max.value })"
          />
        </label>
      </div>
    </ng-template>

    <ng-template #footer let-active>
      <label class="flex items-center gap-2 text-sm">
        <input
          #activeBox
          type="checkbox"
          class="size-4 accent-primary"
          name="active"
          [checked]="active"
          (change)="setActive(activeBox.checked)"
        />
        Active
      </label>
      @if (errors().length > 0) {
        <ul class="list-inside list-disc text-sm text-destructive" role="alert">
          @for (error of errors(); track error) {
            <li>{{ error }}</li>
          }
        </ul>
      }
      <div class="flex gap-2">
        <button hlmBtn type="submit" [disabled]="busy()">Valider</button>
        <button hlmBtn variant="outline" type="button" (click)="cancel()">Annuler</button>
      </div>
    </ng-template>

    <app-confirm-dialog
      [open]="toDelete() !== null"
      [title]="'Supprimer « ' + (toDelete()?.name ?? '') + ' » ?'"
      description="Les réservations déjà faites gardent leurs conditions."
      confirmLabel="Supprimer"
      [destructive]="true"
      (confirmed)="remove()"
      (cancelled)="toDelete.set(null)"
    />
  `,
})
export class OffersPage {
  private readonly api = inject(OffersApi);

  protected readonly kinds = KINDS;
  protected readonly units = UNITS;
  protected readonly promotions = signal<StoredPromotion[]>([]);
  protected readonly options = signal<StoredOption[]>([]);
  protected readonly loadError = signal(false);
  protected readonly editing = signal<Editing | null>(null);
  protected readonly errors = signal<string[]>([]);
  protected readonly busy = signal(false);
  protected readonly toDelete = signal<{
    what: 'promotion' | 'option';
    id: number;
    name: string;
  } | null>(null);

  protected readonly patchPromotionFn = (changes: Partial<PromotionDraft>) =>
    this.patchPromotion(changes);
  protected readonly patchOptionFn = (changes: Partial<OptionDraft>) => this.patchOption(changes);

  constructor() {
    void this.load();
  }

  protected discountText(promotion: Promotion): string {
    const discount = promotion.discount;
    return discount.kind === 'percent'
      ? `−${formatPercent(discount.bp)} %`
      : `−${euros(discount.amount)} ${discount.kind === 'per_stay' ? 'par séjour' : 'par nuit'}`;
  }

  protected nightsText(item: { min_nights: number | null; max_nights: number | null }): string {
    return describeNights(item.min_nights, item.max_nights);
  }

  protected datesText(promotion: Promotion): string {
    const from = promotion.valid_from ? ` du ${formatShort(promotion.valid_from)}` : '';
    const until = promotion.valid_until ? ` au ${formatShort(promotion.valid_until)}` : '';
    return from || until ? ` · arrivées${from}${until}` : '';
  }

  protected priceText(option: StayOption): string {
    return option.price === 0 ? 'offert' : `${euros(option.price)} ${UNIT_TEXT[option.unit]}`;
  }

  protected editPromotion(promotion: StoredPromotion | null): void {
    this.errors.set([]);
    this.editing.set({
      what: 'promotion',
      id: promotion?.id ?? null,
      draft: promotion ? promotionDraft(promotion) : { ...EMPTY_PROMOTION },
    });
  }

  protected editOption(option: StoredOption | null): void {
    this.errors.set([]);
    const nextPosition = Math.max(0, ...this.options().map((o) => o.position + 1));
    this.editing.set({
      what: 'option',
      id: option?.id ?? null,
      draft: option ? optionDraft(option) : { ...EMPTY_OPTION, position: String(nextPosition) },
    });
  }

  protected patchPromotion(changes: Partial<PromotionDraft>): void {
    this.editing.update((e) =>
      e?.what === 'promotion' ? { ...e, draft: { ...e.draft, ...changes } } : e,
    );
  }

  protected patchOption(changes: Partial<OptionDraft>): void {
    this.editing.update((e) =>
      e?.what === 'option' ? { ...e, draft: { ...e.draft, ...changes } } : e,
    );
  }

  protected setActive(active: boolean): void {
    this.editing.update((e) =>
      e?.what === 'promotion'
        ? { ...e, draft: { ...e.draft, active } }
        : e?.what === 'option'
          ? { ...e, draft: { ...e.draft, active } }
          : e,
    );
  }

  protected cancel(): void {
    this.editing.set(null);
    this.errors.set([]);
  }

  protected async save(): Promise<void> {
    const editing = this.editing();
    if (!editing) {
      return;
    }
    let request: Promise<unknown>;
    if (editing.what === 'promotion') {
      const { promotion, errors } = promotionFrom(editing.draft);
      this.errors.set(errors);
      if (!promotion) {
        return;
      }
      request = firstValueFrom<unknown>(
        editing.id === null
          ? this.api.createPromotion(promotion)
          : this.api.updatePromotion(editing.id, promotion),
      );
    } else {
      const { option, errors } = optionFrom(editing.draft);
      this.errors.set(errors);
      if (!option) {
        return;
      }
      request = firstValueFrom<unknown>(
        editing.id === null
          ? this.api.createOption(option)
          : this.api.updateOption(editing.id, option),
      );
    }
    this.busy.set(true);
    try {
      await request;
      this.editing.set(null);
      await this.load();
    } catch (error) {
      this.errors.set(errorMessages(error));
    } finally {
      this.busy.set(false);
    }
  }

  protected askDelete(what: 'promotion' | 'option', id: number, name: string): void {
    this.toDelete.set({ what, id, name });
  }

  protected async remove(): Promise<void> {
    const target = this.toDelete();
    this.toDelete.set(null);
    if (!target) {
      return;
    }
    await firstValueFrom(
      target.what === 'promotion'
        ? this.api.deletePromotion(target.id)
        : this.api.deleteOption(target.id),
    );
    if (this.editing()?.id === target.id && this.editing()?.what === target.what) {
      this.editing.set(null);
    }
    await this.load();
  }

  private async load(): Promise<void> {
    try {
      const offers = await firstValueFrom(this.api.list());
      this.promotions.set(offers.promotions);
      this.options.set(offers.options);
      this.loadError.set(false);
    } catch {
      this.loadError.set(true);
    }
  }
}

function errorMessages(error: unknown): string[] {
  const body = error instanceof HttpErrorResponse ? (error.error as ApiErrorBody | null) : null;
  const details = body?.error?.details ?? [];
  return details.length > 0
    ? details
    : [body?.error?.message ?? 'Une erreur est survenue, réessayez plus tard.'];
}
