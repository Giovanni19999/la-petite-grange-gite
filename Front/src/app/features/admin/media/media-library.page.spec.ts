import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { Media } from './data-access/media.models';
import { MediaLibraryPage } from './media-library.page';

const URL = '/api/admin/media';

function media(id: number, overrides: Partial<Media> = {}): Media {
  return {
    id,
    width: 1920,
    height: 1280,
    sources: [
      { width: 480, url: `/uploads/h${id}-480.webp` },
      { width: 960, url: `/uploads/h${id}-960.webp` },
    ],
    alt: { fr: '', en: '' },
    caption: { fr: '', en: '' },
    focal: { x: 50, y: 50 },
    missing_alt: true,
    used_in: [],
    created_at: '2026-11-20T10:00:00Z',
    ...overrides,
  };
}

describe('F003 media library page', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([{ path: 'admin/medias', component: MediaLibraryPage }]),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
    await harness.navigateByUrl('/admin/medias');
    http.expectOne(URL).flush([
      media(2),
      media(1, {
        alt: { fr: 'La grange', en: '' },
        missing_alt: false,
        used_in: [{ page_id: 1, title: 'La Petite Grange' }],
      }),
    ]);
    await settle();
  });

  afterEach(() => http.verify());

  const page = () => harness.routeNativeElement as HTMLElement;
  const tiles = () =>
    Array.from(page().querySelectorAll<HTMLButtonElement>('[data-testid="media-tile"]'));
  const button = (text: string) =>
    Array.from(page().querySelectorAll<HTMLButtonElement>('button')).find(
      (candidate) => candidate.textContent?.trim() === text,
    )!;

  it('F003-S01 uploads several photos at once and shows them first', async () => {
    const input = page().querySelector<HTMLInputElement>('input[type="file"]')!;
    expect(input.multiple).toBe(true);
    expect(input.accept).toBe('image/jpeg,image/png,image/webp');
    const files = [
      new File(['a'], 'a.jpg', { type: 'image/jpeg' }),
      new File(['b'], 'anim.gif', { type: 'image/gif' }),
    ];
    Object.defineProperty(input, 'files', { value: files });
    input.dispatchEvent(new Event('change'));

    const request = http.expectOne(URL);
    expect(request.request.method).toBe('POST');
    const body = request.request.body as FormData;
    expect(body.getAll('files')).toHaveLength(2);
    request.flush({
      uploaded: [media(3)],
      refused: [{ file_name: 'anim.gif', reason: 'format non accepté' }],
    });
    await settle();

    expect(tiles()).toHaveLength(3);
    expect(tiles()[0].querySelector('img')?.getAttribute('src')).toBe('/uploads/h3-480.webp');
    expect(page().querySelector('[role="alert"]')?.textContent).toContain('anim.gif');
  });

  it('F003-S05 describes a photo and sets its focal point', async () => {
    expect(tiles()[0].textContent).toContain('texte alternatif manquant');
    tiles()[0].click();
    await settle();

    const [altFr] = Array.from(page().querySelectorAll<HTMLInputElement>('aside input'));
    altFr.value = 'Le jardin';
    altFr.dispatchEvent(new Event('input'));
    const area = page().querySelector<HTMLElement>('[data-testid="focal-area"]')!;
    area.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft' }));
    await settle();
    area.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown' }));
    await settle();
    button('Enregistrer').click();

    const request = http.expectOne(`${URL}/2`);
    expect(request.request.method).toBe('PUT');
    expect(request.request.body).toEqual({
      alt: { fr: 'Le jardin', en: '' },
      caption: { fr: '', en: '' },
      focal: { x: 45, y: 55 },
    });
    request.flush(media(2, { alt: { fr: 'Le jardin', en: '' }, missing_alt: false }));
    await settle();

    expect(tiles()[0].textContent).not.toContain('texte alternatif manquant');
    expect(page().querySelector('aside [role="status"]')?.textContent).toContain('Enregistré');
  });

  it('F003-S07 a photo used in a page cannot be deleted and names the page', async () => {
    tiles()[1].click();
    await settle();
    expect(page().querySelector('aside')?.textContent).toContain('La Petite Grange');
    expect(button('Supprimer la photo').disabled).toBe(true);
  });

  it('F003-S06 deletes an unused photo after confirmation', async () => {
    tiles()[0].click();
    await settle();
    button('Supprimer la photo').click();
    await settle();
    (document.querySelector('[data-slot="alert-dialog-action"]') as HTMLButtonElement).click();
    const request = http.expectOne(`${URL}/2`);
    expect(request.request.method).toBe('DELETE');
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    expect(tiles()).toHaveLength(1);
  });
});

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
