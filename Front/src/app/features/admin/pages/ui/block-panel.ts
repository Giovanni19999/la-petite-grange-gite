import { NgTemplateOutlet } from '@angular/common';
import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { HlmInputImports } from '@spartan-ng/helm/input';
import { HlmLabelImports } from '@spartan-ng/helm/label';

import { Lang } from '../../../public/pages/data-access/pages.models';
import { RichTextEditor } from '../../../../shared/rich-text/rich-text-editor';
import { DraftBlock, DraftColumnItem, Localized } from '../data-access/admin-pages.models';
import { blockLabel, emptyText, isFallback } from '../data-access/page-content';
import { Choice, ChoiceGroup } from './choice-group';

const ALIGNS: readonly Choice<'left' | 'center' | 'right'>[] = [
  { value: 'left', label: 'Gauche' },
  { value: 'center', label: 'Centre' },
  { value: 'right', label: 'Droite' },
];
const HEIGHTS: readonly Choice<'small' | 'medium' | 'large'>[] = [
  { value: 'small', label: 'Petite' },
  { value: 'medium', label: 'Moyenne' },
  { value: 'large', label: 'Grande' },
];
const WIDTHS: readonly Choice<'narrow' | 'wide'>[] = [
  { value: 'narrow', label: 'Étroite' },
  { value: 'wide', label: 'Large' },
];
const IMAGE_POSITIONS: readonly Choice<'left' | 'right'>[] = [
  { value: 'left', label: 'Image à gauche' },
  { value: 'right', label: 'Image à droite' },
];
const RATIOS: readonly Choice<'50-50' | '33-67'>[] = [
  { value: '50-50', label: 'Moitié / moitié' },
  { value: '33-67', label: 'Texte plus large' },
];
const GALLERY_COLUMNS: readonly Choice<number>[] = [
  { value: 2, label: '2' },
  { value: 3, label: '3' },
  { value: 4, label: '4' },
];
const BACKGROUNDS: readonly Choice<'background' | 'muted' | 'primary'>[] = [
  { value: 'background', label: 'Blanc' },
  { value: 'muted', label: 'Gris clair' },
  { value: 'primary', label: 'Couleur du site' },
];
const SPACINGS: readonly Choice<'small' | 'medium' | 'large'>[] = [
  { value: 'small', label: 'Petit' },
  { value: 'medium', label: 'Moyen' },
  { value: 'large', label: 'Grand' },
];

/** Limits checked by the API (F002): hero darkening in percent, 2 to 4 columns. */
const MAX_DARKEN = 80;
const MIN_COLUMNS = 2;
const MAX_COLUMNS = 4;

/** Texts and settings of the selected block, in the language being edited (F023). */
@Component({
  selector: 'app-block-panel',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [NgTemplateOutlet, HlmInputImports, HlmLabelImports, ChoiceGroup, RichTextEditor],
  template: `
    @let b = block();
    <div class="flex flex-col gap-5">
      <p class="text-sm text-muted-foreground">
        Bloc « {{ label(b.type) }} » · textes en {{ lang() === 'fr' ? 'français' : 'anglais' }}
      </p>

      @switch (b.type) {
        @case ('hero') {
          <label hlmLabel class="flex flex-col items-start gap-2">
            <span
              >Titre
              @if (untranslated(b.title)) {
                <ng-container *ngTemplateOutlet="todo" />
              }
            </span>
            <input
              #title
              hlmInput
              class="w-full"
              [placeholder]="b.title.fr"
              [value]="b.title[lang()]"
              (input)="patch({ title: text(b.title, title.value) })"
            />
          </label>
          <label hlmLabel class="flex flex-col items-start gap-2">
            Sous-titre
            <input
              #subtitle
              hlmInput
              class="w-full"
              [placeholder]="b.subtitle.fr"
              [value]="b.subtitle[lang()]"
              (input)="patch({ subtitle: text(b.subtitle, subtitle.value) })"
            />
          </label>
          <label hlmLabel class="flex flex-col items-start gap-2">
            Texte du bouton
            <input
              #buttonLabel
              hlmInput
              class="w-full"
              [placeholder]="b.button_label.fr"
              [value]="b.button_label[lang()]"
              (input)="patch({ button_label: text(b.button_label, buttonLabel.value) })"
            />
          </label>
          <label hlmLabel class="flex flex-col items-start gap-2">
            Lien du bouton (commun aux deux langues)
            <input
              #buttonLink
              hlmInput
              class="w-full"
              placeholder="#reserver, /contact ou https://…"
              [value]="b.button_link ?? ''"
              (input)="patch({ button_link: buttonLink.value.trim() || null })"
            />
          </label>
          <p class="rounded-md bg-muted p-3 text-sm text-muted-foreground">
            Image : bientôt disponible avec la médiathèque.
          </p>
          <app-choice-group
            legend="Position du texte"
            [choices]="aligns"
            [value]="b.align"
            (changed)="patch({ align: $event })"
          />
          <app-choice-group
            legend="Hauteur"
            [choices]="heights"
            [value]="b.height"
            (changed)="patch({ height: $event })"
          />
          <label hlmLabel class="flex flex-col items-start gap-2">
            Assombrir l’image : {{ b.darken }} %
            <input
              #darken
              class="w-full accent-primary"
              type="range"
              min="0"
              step="10"
              [max]="maxDarken"
              [value]="b.darken"
              (input)="patch({ darken: +darken.value })"
            />
          </label>
        }
        @case ('text') {
          <div class="flex flex-col gap-2">
            <span class="text-sm font-medium">
              Texte
              @if (untranslated(b.body)) {
                <ng-container *ngTemplateOutlet="todo" />
              }
            </span>
            <app-rich-text-editor
              label="Texte"
              [value]="b.body[lang()]"
              (valueChange)="patch({ body: text(b.body, $event) })"
            />
          </div>
          <app-choice-group
            legend="Largeur"
            [choices]="widths"
            [value]="b.width"
            (changed)="patch({ width: $event })"
          />
          <app-choice-group
            legend="Alignement"
            [choices]="aligns"
            [value]="b.align"
            (changed)="patch({ align: $event })"
          />
        }
        @case ('text_image') {
          <div class="flex flex-col gap-2">
            <span class="text-sm font-medium">
              Texte
              @if (untranslated(b.body)) {
                <ng-container *ngTemplateOutlet="todo" />
              }
            </span>
            <app-rich-text-editor
              label="Texte"
              [value]="b.body[lang()]"
              (valueChange)="patch({ body: text(b.body, $event) })"
            />
          </div>
          <p class="rounded-md bg-muted p-3 text-sm text-muted-foreground">
            Image : bientôt disponible avec la médiathèque.
          </p>
          <app-choice-group
            legend="Position de l’image"
            [choices]="imagePositions"
            [value]="b.image_position"
            (changed)="patch({ image_position: $event })"
          />
          <app-choice-group
            legend="Proportions"
            [choices]="ratios"
            [value]="b.ratio"
            (changed)="patch({ ratio: $event })"
          />
        }
        @case ('columns') {
          @for (item of b.items; track $index; let i = $index) {
            <fieldset class="flex flex-col gap-3 rounded-md border p-3">
              <legend class="px-1 text-sm font-medium">Colonne {{ i + 1 }}</legend>
              <label hlmLabel class="flex flex-col items-start gap-2">
                <span
                  >Titre
                  @if (untranslated(item.title)) {
                    <ng-container *ngTemplateOutlet="todo" />
                  }
                </span>
                <input
                  #columnTitle
                  hlmInput
                  class="w-full"
                  [placeholder]="item.title.fr"
                  [value]="item.title[lang()]"
                  (input)="setColumn(b.items, i, { title: text(item.title, columnTitle.value) })"
                />
              </label>
              <app-rich-text-editor
                [label]="'Texte de la colonne ' + (i + 1)"
                [value]="item.body[lang()]"
                (valueChange)="setColumn(b.items, i, { body: text(item.body, $event) })"
              />
              @if (b.items.length > minColumns) {
                <button
                  type="button"
                  class="self-start text-sm text-destructive hover:underline"
                  (click)="patch({ items: without(b.items, i) })"
                >
                  Retirer la colonne {{ i + 1 }}
                </button>
              }
            </fieldset>
          }
          @if (b.items.length < maxColumns) {
            <button
              type="button"
              class="self-start text-sm text-primary hover:underline"
              (click)="patch({ items: withNewColumn(b.items) })"
            >
              + Ajouter une colonne
            </button>
          }
        }
        @case ('gallery') {
          <p class="rounded-md bg-muted p-3 text-sm text-muted-foreground">
            Photos : bientôt disponibles avec la médiathèque.
          </p>
          <app-choice-group
            legend="Colonnes sur ordinateur"
            [choices]="galleryColumns"
            [value]="b.columns"
            (changed)="patch({ columns: $event })"
          />
        }
      }

      <hr />
      <app-choice-group
        legend="Fond"
        [choices]="backgrounds"
        [value]="b.background"
        (changed)="patch({ background: $event })"
      />
      <app-choice-group
        legend="Espacement"
        [choices]="spacings"
        [value]="b.spacing"
        (changed)="patch({ spacing: $event })"
      />
      <label class="flex items-center gap-2 text-sm">
        <input
          #visible
          type="checkbox"
          class="size-4 accent-primary"
          [checked]="b.visible"
          (change)="patch({ visible: visible.checked })"
        />
        Afficher ce bloc sur le site
      </label>
      <label class="flex items-center gap-2 text-sm">
        <input
          #hideOnMobile
          type="checkbox"
          class="size-4 accent-primary"
          [checked]="b.hide_on_mobile"
          (change)="patch({ hide_on_mobile: hideOnMobile.checked })"
        />
        Masquer sur téléphone
      </label>
    </div>

    <ng-template #todo>
      <span class="ml-1 rounded-full bg-amber-100 px-2 py-0.5 text-xs text-amber-900">
        non traduit
      </span>
    </ng-template>
  `,
})
export class BlockPanel {
  readonly block = input.required<DraftBlock>();
  readonly lang = input.required<Lang>();
  readonly changed = output<DraftBlock>();

  protected readonly aligns = ALIGNS;
  protected readonly heights = HEIGHTS;
  protected readonly widths = WIDTHS;
  protected readonly imagePositions = IMAGE_POSITIONS;
  protected readonly ratios = RATIOS;
  protected readonly galleryColumns = GALLERY_COLUMNS;
  protected readonly backgrounds = BACKGROUNDS;
  protected readonly spacings = SPACINGS;
  protected readonly maxDarken = MAX_DARKEN;
  protected readonly minColumns = MIN_COLUMNS;
  protected readonly maxColumns = MAX_COLUMNS;
  protected readonly label = blockLabel;

  protected patch(changes: Partial<DraftBlock>): void {
    this.changed.emit({ ...this.block(), ...changes } as DraftBlock);
  }

  /** The localized text with the language being edited replaced. */
  protected text(current: Localized, value: string): Localized {
    return { ...current, [this.lang()]: value };
  }

  protected untranslated(text: Localized): boolean {
    return isFallback(text, this.lang());
  }

  protected setColumn(
    items: DraftColumnItem[],
    index: number,
    changes: Partial<DraftColumnItem>,
  ): void {
    this.patch({ items: items.map((item, i) => (i === index ? { ...item, ...changes } : item)) });
  }

  protected without(items: DraftColumnItem[], index: number): DraftColumnItem[] {
    return items.filter((_, i) => i !== index);
  }

  protected withNewColumn(items: DraftColumnItem[]): DraftColumnItem[] {
    return [...items, { title: emptyText(), body: emptyText() }];
  }
}
