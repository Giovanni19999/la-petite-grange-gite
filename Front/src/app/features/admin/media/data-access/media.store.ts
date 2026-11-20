import { HttpErrorResponse } from '@angular/common/http';
import { Injectable, computed, inject, signal } from '@angular/core';
import { firstValueFrom } from 'rxjs';

import { ApiErrorBody } from '../../pages/data-access/admin-pages.models';
import { MediaApi } from './media.api';
import { Media, MediaDetails, UploadResult } from './media.models';

/**
 * Media library state (F003), shared by the library page and the image picker of the editor.
 */
@Injectable({ providedIn: 'root' })
export class MediaStore {
  private readonly api = inject(MediaApi);
  private readonly items = signal<Media[]>([]);
  private loading: Promise<void> | null = null;

  /** Every media, newest first. */
  readonly all = this.items.asReadonly();
  readonly byId = computed(() => new Map(this.items().map((media) => [media.id, media])));
  readonly loadError = signal(false);
  readonly uploading = signal(false);

  /** Loads the library once (call {@link reload} to refresh). */
  ensureLoaded(): Promise<void> {
    this.loading ??= this.reload();
    return this.loading;
  }

  async reload(): Promise<void> {
    try {
      this.items.set(await firstValueFrom(this.api.list()));
      this.loadError.set(false);
    } catch {
      this.loadError.set(true);
    }
  }

  /** Sends files; the accepted ones are added at the top of the library. */
  async upload(files: readonly File[]): Promise<UploadResult> {
    this.uploading.set(true);
    try {
      const result = await firstValueFrom(this.api.upload(files));
      const added = new Set(result.uploaded.map((media) => media.id));
      this.items.update((items) => [
        ...result.uploaded,
        ...items.filter((media) => !added.has(media.id)),
      ]);
      return result;
    } catch (error) {
      return {
        uploaded: [],
        refused: files.map((file) => ({ file_name: file.name, reason: errorMessage(error) })),
      };
    } finally {
      this.uploading.set(false);
    }
  }

  /** Saves the texts and focal point; returns the error message, or `null`. */
  async update(id: number, details: MediaDetails): Promise<string | null> {
    try {
      const updated = await firstValueFrom(this.api.update(id, details));
      this.items.update((items) => items.map((media) => (media.id === id ? updated : media)));
      return null;
    } catch (error) {
      return errorMessage(error);
    }
  }

  /** Deletes a media; returns the error message (e.g. used in a page), or `null`. */
  async delete(id: number): Promise<string | null> {
    try {
      await firstValueFrom(this.api.delete(id));
      this.items.update((items) => items.filter((media) => media.id !== id));
      return null;
    } catch (error) {
      return errorMessage(error);
    }
  }
}

/** Message of an API error, in French. */
export function errorMessage(error: unknown): string {
  const body = error instanceof HttpErrorResponse ? (error.error as ApiErrorBody | null) : null;
  if (error instanceof HttpErrorResponse && error.status === 413) {
    return 'Envoi trop lourd : envoyez moins de photos à la fois.';
  }
  const details = body?.error?.details ?? [];
  if (details.length > 0) {
    return details.join(' ; ');
  }
  return body?.error?.message ?? 'Une erreur est survenue, réessayez plus tard.';
}
