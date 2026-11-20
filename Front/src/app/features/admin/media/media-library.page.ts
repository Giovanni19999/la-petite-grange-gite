import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { HlmInputImports } from '@spartan-ng/helm/input';

import { ConfirmDialog } from '../../../shared/confirm/confirm-dialog';
import { thumbnailUrl } from './data-access/media.api';
import { Media } from './data-access/media.models';
import { MediaStore } from './data-access/media.store';
import { MediaDetailsPanel } from './ui/media-details-panel';
import { MediaUpload } from './ui/media-upload';

/** Media library (F003): upload, search, describe, delete. Usable on a phone. */
@Component({
  selector: 'app-media-library-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    FormsModule,
    RouterLink,
    HlmButtonImports,
    HlmInputImports,
    ConfirmDialog,
    MediaDetailsPanel,
    MediaUpload,
  ],
  template: `
    <section class="mx-auto flex max-w-5xl flex-col gap-6 px-4 py-8">
      <div>
        <a routerLink="/admin" class="text-sm text-muted-foreground hover:underline">
          ← Espace administrateur
        </a>
        <h1 class="text-2xl font-semibold">Médiathèque</h1>
      </div>

      <app-media-upload (uploaded)="selected.set($event[0].id)" />

      <label class="flex flex-col gap-2 text-sm font-medium">
        Rechercher
        <input
          hlmInput
          type="search"
          class="w-full max-w-sm"
          placeholder="Texte alternatif ou légende"
          [ngModel]="search()"
          (ngModelChange)="search.set($event)"
        />
      </label>

      @if (store.loadError()) {
        <p class="text-destructive" role="alert">
          Impossible de charger la médiathèque, réessayez plus tard.
        </p>
      }

      <div class="flex flex-col gap-6 lg:flex-row">
        <ul
          class="grid flex-1 grid-cols-2 content-start gap-3 sm:grid-cols-3 md:grid-cols-4"
          aria-label="Photos"
        >
          @for (media of visible(); track media.id) {
            <li>
              <button
                type="button"
                class="group relative block w-full overflow-hidden rounded-md border text-left outline-offset-2 focus-visible:outline-2 focus-visible:outline-ring"
                [class.ring-2]="selected() === media.id"
                [class.ring-primary]="selected() === media.id"
                [attr.aria-pressed]="selected() === media.id"
                [attr.aria-label]="label(media)"
                data-testid="media-tile"
                (click)="selected.set(media.id)"
              >
                <img
                  class="aspect-square w-full object-cover"
                  [src]="thumbnail(media)"
                  alt=""
                  loading="lazy"
                  [style.object-position]="media.focal.x + '% ' + media.focal.y + '%'"
                />
                @if (media.missing_alt) {
                  <span
                    class="absolute bottom-1 left-1 rounded bg-amber-100 px-1.5 py-0.5 text-xs text-amber-900"
                  >
                    texte alternatif manquant
                  </span>
                }
              </button>
            </li>
          } @empty {
            <li class="col-span-full text-muted-foreground">
              {{ search() ? 'Aucune photo ne correspond.' : 'Aucune photo pour l’instant.' }}
            </li>
          }
        </ul>

        @if (current(); as media) {
          <aside class="w-full shrink-0 lg:w-96" aria-label="Photo sélectionnée">
            <app-media-details-panel
              [media]="media"
              (deleteRequested)="toDelete.set(media)"
              (closed)="selected.set(null)"
            />
            @if (deleteError(); as message) {
              <p class="mt-2 text-sm text-destructive" role="alert">{{ message }}</p>
            }
          </aside>
        }
      </div>
    </section>

    <app-confirm-dialog
      [open]="toDelete() !== null"
      title="Supprimer cette photo ?"
      description="Elle sera retirée de la médiathèque et ses fichiers supprimés."
      confirmLabel="Supprimer"
      [destructive]="true"
      (confirmed)="remove()"
      (cancelled)="toDelete.set(null)"
    />
  `,
})
export class MediaLibraryPage {
  protected readonly store = inject(MediaStore);
  protected readonly search = signal('');
  protected readonly selected = signal<number | null>(null);
  protected readonly toDelete = signal<Media | null>(null);
  protected readonly deleteError = signal<string | null>(null);

  protected readonly visible = computed(() => {
    const words = this.search().trim().toLowerCase();
    const all = this.store.all();
    if (!words) {
      return all;
    }
    return all.filter((media) =>
      [media.alt.fr, media.alt.en, media.caption.fr, media.caption.en]
        .join(' ')
        .toLowerCase()
        .includes(words),
    );
  });
  protected readonly current = computed(() => {
    const id = this.selected();
    return id === null ? null : (this.store.byId().get(id) ?? null);
  });

  constructor() {
    void this.store.reload();
  }

  protected thumbnail(media: Media): string {
    return thumbnailUrl(media, 320);
  }

  protected label(media: Media): string {
    return media.alt.fr ? `Photo : ${media.alt.fr}` : `Photo n° ${media.id} sans texte alternatif`;
  }

  protected async remove(): Promise<void> {
    const media = this.toDelete();
    this.toDelete.set(null);
    if (!media) {
      return;
    }
    const error = await this.store.delete(media.id);
    this.deleteError.set(error);
    if (!error) {
      this.selected.set(null);
    }
  }
}
