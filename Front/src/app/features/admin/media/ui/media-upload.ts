import { ChangeDetectionStrategy, Component, inject, output, signal } from '@angular/core';

import { MediaStore } from '../data-access/media.store';
import { Media, UploadResult } from '../data-access/media.models';

/** Formats the API accepts (F003). */
const ACCEPTED = 'image/jpeg,image/png,image/webp';

/**
 * Drop zone and file picker (several photos at once, from the phone gallery or camera too).
 * Refused files are listed with their reason; accepted ones are emitted.
 */
@Component({
  selector: 'app-media-upload',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div
      class="flex flex-col items-center gap-2 rounded-lg border-2 border-dashed p-6 text-center transition"
      [class.border-primary]="dragging()"
      [class.bg-primary/5]="dragging()"
      data-testid="drop-zone"
      (dragover)="onDragOver($event)"
      (dragleave)="dragging.set(false)"
      (drop)="onDrop($event)"
    >
      <p class="font-medium">Déposez vos photos ici</p>
      <p class="text-sm text-muted-foreground">JPEG, PNG ou WebP, plusieurs à la fois.</p>
      <label
        class="cursor-pointer rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:opacity-90 has-focus-visible:ring-2 has-focus-visible:ring-ring"
      >
        {{ store.uploading() ? 'Envoi en cours…' : 'Choisir des photos' }}
        <input
          #picker
          class="sr-only"
          type="file"
          multiple
          [accept]="accepted"
          [disabled]="store.uploading()"
          (change)="send(picker.files); picker.value = ''"
        />
      </label>
    </div>
    @if (refused().length > 0) {
      <ul class="mt-2 flex flex-col gap-1 text-sm text-destructive" role="alert">
        @for (file of refused(); track file.file_name) {
          <li>
            <span class="font-medium">{{ file.file_name }}</span> : {{ file.reason }}
          </li>
        }
      </ul>
    }
  `,
})
export class MediaUpload {
  protected readonly store = inject(MediaStore);
  protected readonly accepted = ACCEPTED;
  protected readonly dragging = signal(false);
  protected readonly refused = signal<UploadResult['refused']>([]);

  /** Media accepted by the last upload. */
  readonly uploaded = output<Media[]>();

  protected onDragOver(event: DragEvent): void {
    event.preventDefault();
    this.dragging.set(true);
  }

  protected onDrop(event: DragEvent): void {
    event.preventDefault();
    this.dragging.set(false);
    void this.send(event.dataTransfer?.files ?? null);
  }

  protected async send(list: FileList | null): Promise<void> {
    const files = Array.from(list ?? []);
    if (files.length === 0) {
      return;
    }
    const result = await this.store.upload(files);
    this.refused.set(result.refused);
    if (result.uploaded.length > 0) {
      this.uploaded.emit(result.uploaded);
    }
  }
}
