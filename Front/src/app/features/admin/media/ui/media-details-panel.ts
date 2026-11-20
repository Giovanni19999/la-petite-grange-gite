import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  input,
  output,
  signal,
  untracked,
} from '@angular/core';
import { RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { HlmInputImports } from '@spartan-ng/helm/input';
import { HlmLabelImports } from '@spartan-ng/helm/label';

import { FocalPoint, Media, MediaDetails, MediaText } from '../data-access/media.models';
import { MediaStore } from '../data-access/media.store';
import { FocalPointPicker } from './focal-point-picker';

/** Texts, focal point, pages using the photo, and delete (F003). */
@Component({
  selector: 'app-media-details-panel',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink, HlmButtonImports, HlmInputImports, HlmLabelImports, FocalPointPicker],
  template: `
    @let m = media();
    @let d = draft();
    <form
      class="flex flex-col gap-4 rounded-lg border p-4"
      (submit)="$event.preventDefault(); save()"
    >
      <div class="flex items-center justify-between gap-2">
        <h2 class="font-semibold">Photo n° {{ m.id }}</h2>
        <button hlmBtn variant="ghost" size="sm" type="button" (click)="closed.emit()">
          Fermer
        </button>
      </div>
      <p class="text-xs text-muted-foreground">{{ m.width }} × {{ m.height }} px</p>

      <app-focal-point-picker [media]="m" [value]="d.focal" (changed)="setFocal($event)" />

      @for (lang of langs; track lang) {
        <fieldset class="flex flex-col gap-3">
          <legend class="mb-1 text-sm font-medium">
            {{ lang === 'fr' ? 'Français' : 'Anglais' }}
          </legend>
          <label hlmLabel class="flex flex-col items-start gap-2">
            Texte alternatif (ce que montre la photo)
            <input
              #alt
              hlmInput
              class="w-full"
              maxlength="300"
              [placeholder]="lang === 'en' ? d.alt.fr : 'Ex. : la grange vue du jardin'"
              [value]="d.alt[lang]"
              (input)="setText('alt', lang, alt.value)"
            />
          </label>
          <label hlmLabel class="flex flex-col items-start gap-2">
            Légende (facultative)
            <input
              #caption
              hlmInput
              class="w-full"
              maxlength="300"
              [placeholder]="lang === 'en' ? d.caption.fr : ''"
              [value]="d.caption[lang]"
              (input)="setText('caption', lang, caption.value)"
            />
          </label>
        </fieldset>
      }
      @if (!d.alt.fr.trim()) {
        <p class="text-sm text-amber-900">
          Ajoutez un texte alternatif en français : il décrit la photo aux personnes malvoyantes et
          aux moteurs de recherche.
        </p>
      }

      <div class="flex flex-wrap items-center gap-2">
        <button hlmBtn type="submit" [disabled]="saving()">
          {{ saving() ? 'Enregistrement…' : 'Enregistrer' }}
        </button>
        @if (status(); as text) {
          <span class="text-sm" role="status" [class.text-destructive]="failed()">{{ text }}</span>
        }
      </div>

      <div class="flex flex-col gap-2 border-t pt-3">
        @if (m.used_in.length > 0) {
          <p class="text-sm">Utilisée dans :</p>
          <ul class="flex flex-col gap-1 text-sm">
            @for (page of m.used_in; track page.page_id) {
              <li>
                <a class="underline" [routerLink]="['/admin/pages', page.page_id]">{{
                  page.title
                }}</a>
              </li>
            }
          </ul>
          <p class="text-xs text-muted-foreground">
            Retirez-la de ces pages pour pouvoir la supprimer.
          </p>
        }
        <button
          hlmBtn
          variant="outline"
          size="sm"
          type="button"
          class="self-start text-destructive"
          [disabled]="m.used_in.length > 0"
          (click)="deleteRequested.emit()"
        >
          Supprimer la photo
        </button>
      </div>
    </form>
  `,
})
export class MediaDetailsPanel {
  private readonly store = inject(MediaStore);

  readonly media = input.required<Media>();
  readonly deleteRequested = output<void>();
  readonly closed = output<void>();

  protected readonly langs = ['fr', 'en'] as const;
  private readonly mediaId = computed(() => this.media().id);
  protected readonly draft = signal<MediaDetails>(emptyDetails());
  protected readonly saving = signal(false);
  protected readonly status = signal<string | null>(null);
  protected readonly failed = signal(false);

  constructor() {
    // Another photo selected: start from its saved texts (not after saving this one).
    effect(() => {
      this.mediaId();
      const media = untracked(this.media);
      this.draft.set({
        alt: { ...media.alt },
        caption: { ...media.caption },
        focal: { ...media.focal },
      });
      this.status.set(null);
    });
  }

  protected setText(field: 'alt' | 'caption', lang: keyof MediaText, value: string): void {
    this.draft.update((draft) => ({ ...draft, [field]: { ...draft[field], [lang]: value } }));
  }

  protected setFocal(focal: FocalPoint): void {
    this.draft.update((draft) => ({ ...draft, focal }));
  }

  protected async save(): Promise<void> {
    if (this.saving()) {
      return;
    }
    this.saving.set(true);
    const error = await this.store.update(this.media().id, this.draft());
    this.saving.set(false);
    this.failed.set(error !== null);
    this.status.set(error ?? 'Enregistré');
  }
}

function emptyDetails(): MediaDetails {
  return { alt: { fr: '', en: '' }, caption: { fr: '', en: '' }, focal: { x: 50, y: 50 } };
}
