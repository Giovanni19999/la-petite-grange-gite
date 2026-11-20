import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter, withComponentInputBinding } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { AdminPage, DraftHeroBlock } from '../data-access/admin-pages.models';
import { newBlock, newPageContent } from '../data-access/page-content';
import { Media } from '../../media/data-access/media.models';
import { PageEditorPage } from './page-editor.page';

const PAGE_URL = '/api/admin/pages/1';
const MEDIA_URL = '/api/admin/media';

/** Media library of the tests: one described photo (F003). */
const MEDIA: Media[] = [
  {
    id: 7,
    width: 1920,
    height: 1280,
    sources: [
      { width: 480, url: '/uploads/abc-480.webp' },
      { width: 960, url: '/uploads/abc-960.webp' },
    ],
    alt: { fr: 'La grange au soleil', en: '' },
    caption: { fr: '', en: '' },
    focal: { x: 30, y: 70 },
    missing_alt: false,
    used_in: [],
    created_at: '2026-11-20T10:00:00Z',
  },
  {
    id: 8,
    width: 1600,
    height: 1000,
    sources: [{ width: 480, url: '/uploads/def-480.webp' }],
    alt: { fr: 'Le jardin', en: '' },
    caption: { fr: '', en: '' },
    focal: { x: 50, y: 50 },
    missing_alt: false,
    used_in: [],
    created_at: '2026-11-20T10:00:00Z',
  },
];

function adminPage(): AdminPage {
  const draft = {
    ...newPageContent('Accueil', 'accueil', 0),
    blocks: [
      {
        ...(newBlock('hero', new Set()) as DraftHeroBlock),
        id: 'hero',
        title: { fr: 'La Petite Grange', en: '' },
      },
      { ...newBlock('text', new Set()), id: 'intro', hide_on_mobile: true },
      { ...newBlock('columns', new Set()), id: 'atouts' },
    ],
  };
  return {
    id: 1,
    is_home: true,
    draft_version: 4,
    draft,
    published: draft,
    published_at: '2026-11-20T10:00:00Z',
  };
}

function mockScreen(wide: boolean): void {
  vi.stubGlobal(
    'matchMedia',
    (query: string) =>
      ({
        matches: wide,
        media: query,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
      }) as unknown as MediaQueryList,
  );
}

describe('F023 page editor', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  async function open(wide = true): Promise<void> {
    mockScreen(wide);
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter(
          [{ path: 'admin/pages/:id', component: PageEditorPage }],
          withComponentInputBinding(),
        ),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
    await harness.navigateByUrl('/admin/pages/1');
    http.expectOne(PAGE_URL).flush(adminPage());
    http.expectOne(MEDIA_URL).flush(MEDIA);
    await settle();
  }

  afterEach(() => {
    http.verify();
    vi.unstubAllGlobals();
  });

  const root = () => harness.routeNativeElement as HTMLElement;
  const previewBlocks = () =>
    Array.from(root().querySelectorAll<HTMLElement>('[data-testid="preview-block"]'));
  const button = (text: string, scope: ParentNode = root()) =>
    Array.from(scope.querySelectorAll<HTMLButtonElement>('button')).find(
      (candidate) =>
        candidate.textContent?.trim() === text || candidate.getAttribute('aria-label') === text,
    )!;

  it('F023-S03 the preview draws the draft with the public block components', async () => {
    await open();
    expect(previewBlocks().map((block) => block.dataset['blockId'])).toEqual([
      'hero',
      'intro',
      'atouts',
    ]);
    expect(
      root().querySelector('[data-testid="preview"] app-hero-block h1')?.textContent,
    ).toContain('La Petite Grange');
    expect(root().querySelectorAll('[data-testid="preview"] app-block-renderer')).toHaveLength(3);
  });

  it('F023-S05 blocks can be moved, duplicated and added with the keyboard-accessible buttons', async () => {
    await open();
    button('Descendre', previewBlocks()[0]).click();
    await settle();
    expect(previewBlocks().map((block) => block.dataset['blockId'])).toEqual([
      'intro',
      'hero',
      'atouts',
    ]);

    button('Dupliquer', previewBlocks()[0]).click();
    await settle();
    expect(previewBlocks()).toHaveLength(4);

    button('Ajouter un bloc à la position 1').click();
    await settle();
    Array.from(root().querySelectorAll<HTMLButtonElement>('[aria-label="Type de bloc"] button'))
      .find((candidate) => candidate.querySelector('span')?.textContent === 'Texte')!
      .click();
    await settle();
    expect(previewBlocks()).toHaveLength(5);
    expect(previewBlocks()[0].dataset['blockId']).toMatch(/^text-/);
    expect(root().querySelector('[data-testid="save-status"]')?.textContent).toContain(
      'Modifications en cours',
    );
    // The pending autosave is not awaited here.
    http.match(() => true);
  });

  it('F023-S07 in English, a missing text is shown in French, greyed out', async () => {
    await open();
    button('EN').click();
    await settle();
    const title = root().querySelector('[data-testid="preview"] app-hero-block h1') as HTMLElement;
    expect(title.textContent).toContain('La Petite Grange');
    expect(title.classList).toContain('opacity-60');
    expect(root().querySelector('[data-testid="preview"]')?.getAttribute('lang')).toBe('en');
  });

  it('F023-S09 the phone preview is narrow and hides "hide on mobile" blocks', async () => {
    await open();
    button('Téléphone').click();
    await settle();
    const preview = root().querySelector<HTMLElement>('[data-testid="preview"]')!;
    expect(preview.style.maxWidth).toBe('375px');
    expect(preview.classList).toContain('@container');
    const frame = previewBlocks()[1].querySelector('app-block-frame') as HTMLElement;
    expect(frame.className).toContain('@max-3xl:hidden');
  });

  it('F023-S15 on a narrow screen the block panel opens as a bottom drawer', async () => {
    await open(false);
    expect(root().querySelector('aside')).toBeNull();
    const hero = previewBlocks()[0].querySelector<HTMLElement>('[role="button"]')!;
    expect(hero.tabIndex).toBe(0);
    hero.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    await settle();
    const drawer = document.querySelector('hlm-sheet-content');
    expect(drawer?.getAttribute('data-side')).toBe('bottom');
    expect(drawer?.textContent).toContain('Bandeau');
  });

  it('F003-S08 a photo chosen for the hero is shown in the preview', async () => {
    await open();
    previewBlocks()[0].querySelector<HTMLElement>('[role="button"]')!.click();
    await settle();
    button('Choisir une image', root().querySelector('aside')!).click();
    await settle();
    (document.querySelector('[data-testid="picker-tile"]') as HTMLButtonElement).click();
    await settle();

    const img = previewBlocks()[0].querySelector('img')!;
    expect(img.getAttribute('srcset')).toBe(
      '/uploads/abc-480.webp 480w, /uploads/abc-960.webp 960w',
    );
    expect(img.getAttribute('alt')).toBe('La grange au soleil');
    expect(img.style.objectPosition).toBe('30% 70%');
    expect(root().querySelector('[data-testid="save-status"]')?.textContent).toContain(
      'Modifications en cours',
    );
    http.match(() => true);
  });

  it('F024-S04 S05 adds a carousel block and ticks its photos', async () => {
    await open();
    button('Ajouter un bloc à la position 4').click();
    await settle();
    Array.from(root().querySelectorAll<HTMLButtonElement>('[aria-label="Type de bloc"] button'))
      .find((candidate) => candidate.querySelector('span')?.textContent === 'Carrousel')!
      .click();
    await settle();

    button('+ Ajouter des photos', root().querySelector('aside')!).click();
    await settle();
    const tiles = Array.from(
      document.querySelectorAll<HTMLButtonElement>('[data-testid="picker-tile"]'),
    );
    tiles[1].click();
    tiles[0].click();
    await settle();
    Array.from(document.querySelectorAll<HTMLButtonElement>('hlm-dialog-content button'))
      .find((candidate) => candidate.textContent?.trim().startsWith('Ajouter'))!
      .click();
    await settle();

    const carousel = previewBlocks()[3].querySelector('app-carousel')!;
    expect(carousel.querySelectorAll('[data-testid="slide"]')).toHaveLength(2);
    expect(carousel.querySelector('img')?.getAttribute('alt')).toBe('Le jardin');
    expect(carousel.querySelector('[data-testid="pause"]')).toBeNull();
    http.match(() => true);
  });

  it('F023-S12 publishing asks for confirmation with the summary, focus on "Annuler"', async () => {
    await open();
    const hero = previewBlocks()[0].querySelector<HTMLElement>('[role="button"]')!;
    hero.click();
    await settle();
    const title = root().querySelector<HTMLInputElement>('aside input')!;
    title.value = 'Nouveau titre';
    title.dispatchEvent(new Event('input'));
    await settle();

    button('Publier').click();
    await settle();
    http.expectOne(`${PAGE_URL}/draft`).flush({ draft_version: 5 });
    await settle();
    http.expectOne(`${PAGE_URL}/changes`).flush({
      draft_version: 5,
      nothing_to_publish: false,
      changes: {
        settings_changed: false,
        blocks_added: ['atouts'],
        blocks_changed: ['hero'],
        blocks_removed: [],
        blocks_moved: false,
        languages: ['fr'],
      },
    });
    await settle();

    const summary = document.querySelector('[data-testid="publish-summary"]');
    expect(summary?.textContent).toContain('Bloc ajouté');
    expect(summary?.textContent).toContain('Colonnes');
    expect(summary?.textContent).toContain('Bloc modifié');
    expect(summary?.textContent).toContain('Bandeau');
    expect(
      document.querySelector('[data-slot="alert-dialog-cancel"]')?.hasAttribute('data-autofocus'),
    ).toBe(true);

    (document.querySelector('[data-slot="alert-dialog-action"]') as HTMLButtonElement).click();
    const publish = http.expectOne(`${PAGE_URL}/publish`);
    expect(publish.request.body).toEqual({ draft_version: 5 });
    publish.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    http.expectOne(PAGE_URL).flush(adminPage());
    await settle();
    expect(root().textContent).toContain('Page publiée');
  });
});

/** Lets pending promises (HTTP responses turned into promises) finish, then renders. */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
