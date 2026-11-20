import {
  ChangeDetectionStrategy,
  Component,
  effect,
  inject,
  input,
  output,
  signal,
} from '@angular/core';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { HlmDialogImports } from '@spartan-ng/helm/dialog';

import { thumbnailUrl } from '../data-access/media.api';
import { Media } from '../data-access/media.models';
import { MediaStore } from '../data-access/media.store';
import { MediaUpload } from './media-upload';

/**
 * Chooses photos of the media library for a block (F003), or uploads new ones on the spot.
 * In single mode a click chooses; in multiple mode photos are ticked then added.
 */
@Component({
  selector: 'app-media-picker-dialog',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [HlmButtonImports, HlmDialogImports, MediaUpload],
  template: `
    <hlm-dialog [state]="open() ? 'open' : 'closed'" (closed)="closed.emit()">
      <hlm-dialog-content
        *hlmDialogPortal="let ctx"
        class="max-h-[90vh] w-[calc(100vw-2rem)] max-w-3xl overflow-y-auto"
      >
        <hlm-dialog-header>
          <h2 hlmDialogTitle>{{ multiple() ? 'Choisir des photos' : 'Choisir une photo' }}</h2>
          <p hlmDialogDescription>
            {{
              multiple()
                ? 'Cochez les photos à ajouter, dans l’ordre voulu.'
                : 'Cliquez sur la photo à utiliser.'
            }}
          </p>
        </hlm-dialog-header>

        <app-media-upload (uploaded)="onUploaded($event)" />

        <ul class="grid grid-cols-3 gap-2 sm:grid-cols-4" aria-label="Photos de la médiathèque">
          @for (media of store.all(); track media.id) {
            <li>
              <button
                type="button"
                class="relative block w-full overflow-hidden rounded-md border outline-offset-2 focus-visible:outline-2 focus-visible:outline-ring"
                [class.ring-2]="isTicked(media)"
                [class.ring-primary]="isTicked(media)"
                [attr.aria-pressed]="multiple() ? isTicked(media) : null"
                [attr.aria-label]="media.alt.fr || 'Photo n° ' + media.id"
                data-testid="picker-tile"
                (click)="choose(media)"
              >
                <img
                  class="aspect-square w-full object-cover"
                  [src]="thumbnail(media)"
                  alt=""
                  loading="lazy"
                  [style.object-position]="media.focal.x + '% ' + media.focal.y + '%'"
                />
                @if (multiple() && isTicked(media)) {
                  <span
                    class="absolute top-1 right-1 flex size-6 items-center justify-center rounded-full bg-primary text-xs text-primary-foreground"
                  >
                    {{ ticked().indexOf(media.id) + 1 }}
                  </span>
                }
              </button>
            </li>
          } @empty {
            <li class="col-span-full text-sm text-muted-foreground">
              La médiathèque est vide : envoyez une première photo.
            </li>
          }
        </ul>

        @if (multiple()) {
          <hlm-dialog-footer>
            <button hlmBtn variant="outline" type="button" (click)="closed.emit()">Annuler</button>
            <button hlmBtn type="button" [disabled]="ticked().length === 0" (click)="confirm()">
              Ajouter {{ ticked().length || '' }}
            </button>
          </hlm-dialog-footer>
        }
      </hlm-dialog-content>
    </hlm-dialog>
  `,
})
export class MediaPickerDialog {
  protected readonly store = inject(MediaStore);

  readonly open = input(false);
  /** Several photos (gallery) or one (hero, text and image). */
  readonly multiple = input(false);
  /** Ids chosen, in order. */
  readonly chosen = output<number[]>();
  readonly closed = output<void>();

  protected readonly ticked = signal<number[]>([]);

  constructor() {
    effect(() => {
      if (this.open()) {
        this.ticked.set([]);
        void this.store.ensureLoaded();
      }
    });
  }

  protected thumbnail(media: Media): string {
    return thumbnailUrl(media, 320);
  }

  protected isTicked(media: Media): boolean {
    return this.ticked().includes(media.id);
  }

  protected choose(media: Media): void {
    if (!this.multiple()) {
      this.chosen.emit([media.id]);
      return;
    }
    this.ticked.update((ids) =>
      ids.includes(media.id) ? ids.filter((id) => id !== media.id) : [...ids, media.id],
    );
  }

  protected confirm(): void {
    this.chosen.emit(this.ticked());
  }

  /** Photos sent from the picker are chosen at once (single) or ticked (multiple). */
  protected onUploaded(uploaded: Media[]): void {
    if (!this.multiple()) {
      this.chosen.emit([uploaded[0].id]);
      return;
    }
    this.ticked.update((ids) => [
      ...ids,
      ...uploaded.map((media) => media.id).filter((id) => !ids.includes(id)),
    ]);
  }
}
