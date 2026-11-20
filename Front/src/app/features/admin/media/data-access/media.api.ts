import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

import { PublicImage } from '../../../public/pages/data-access/pages.models';
import { Media, MediaDetails, UploadResult } from './media.models';

const BASE = '/api/admin/media';

/** Media library API (F003). Requires the owner's session. */
@Injectable({ providedIn: 'root' })
export class MediaApi {
  private readonly http = inject(HttpClient);

  list(): Observable<Media[]> {
    return this.http.get<Media[]>(BASE);
  }

  /** Sends several files at once; each one is accepted or refused on its own. */
  upload(files: readonly File[]): Observable<UploadResult> {
    const form = new FormData();
    for (const file of files) {
      form.append('files', file, file.name);
    }
    return this.http.post<UploadResult>(BASE, form);
  }

  update(id: number, details: MediaDetails): Observable<Media> {
    return this.http.put<Media>(`${BASE}/${id}`, details);
  }

  delete(id: number): Observable<void> {
    return this.http.delete<void>(`${BASE}/${id}`);
  }
}

/** URL of the smallest version at least `minWidth` wide (or the largest one). */
export function thumbnailUrl(media: Pick<Media, 'sources'>, minWidth = 320): string {
  const sources = media.sources;
  return (sources.find((source) => source.width >= minWidth) ?? sources.at(-1))?.url ?? '';
}

/** The media as the public pages receive it, in `lang` (English falls back to French). */
export function toPublicImage(media: Media, lang: 'fr' | 'en'): PublicImage {
  const text = (value: { fr: string; en: string }) =>
    lang === 'en' && value.en.trim() !== '' ? value.en : value.fr;
  return {
    id: media.id,
    width: media.width,
    height: media.height,
    sources: media.sources,
    alt: text(media.alt),
    caption: text(media.caption),
    focal_x: media.focal.x,
    focal_y: media.focal.y,
  };
}
