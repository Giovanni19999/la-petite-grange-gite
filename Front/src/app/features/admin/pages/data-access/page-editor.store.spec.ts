import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';

import { AdminPage, DraftHeroBlock, PageChanges } from './admin-pages.models';
import { newBlock, newPageContent } from './page-content';
import { AUTOSAVE_DELAY_MS, PageEditorStore } from './page-editor.store';

const PAGE_URL = '/api/admin/pages/1';

function adminPage(draftVersion = 4): AdminPage {
  const draft = {
    ...newPageContent('Accueil', 'accueil', 0),
    blocks: [
      {
        ...(newBlock('hero', new Set()) as DraftHeroBlock),
        id: 'hero',
        title: { fr: 'La Petite Grange', en: '' },
      },
    ],
  };
  return {
    id: 1,
    is_home: true,
    draft_version: draftVersion,
    draft,
    published: draft,
    published_at: '2026-11-20T10:00:00Z',
  };
}

const CHANGES: PageChanges = {
  draft_version: 5,
  nothing_to_publish: false,
  changes: {
    settings_changed: false,
    blocks_added: [],
    blocks_changed: ['hero'],
    blocks_removed: [],
    blocks_moved: false,
    languages: ['fr'],
  },
};

describe('F023 page editor store', () => {
  let store: PageEditorStore;
  let http: HttpTestingController;

  beforeEach(async () => {
    vi.useFakeTimers();
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting(), PageEditorStore],
    });
    store = TestBed.inject(PageEditorStore);
    http = TestBed.inject(HttpTestingController);
    const loading = store.load(1);
    http.expectOne(PAGE_URL).flush(adminPage());
    await loading;
  });

  afterEach(() => {
    http.verify();
    vi.useRealTimers();
  });

  function changeTitle(title: string): void {
    const hero = store.content()!.blocks[0] as DraftHeroBlock;
    store.updateBlock({ ...hero, title: { ...hero.title, fr: title } });
  }

  async function autosave(): Promise<void> {
    await vi.advanceTimersByTimeAsync(AUTOSAVE_DELAY_MS);
  }

  it('F023-S04 saves the draft automatically with the version it was based on', async () => {
    changeTitle('Nouveau titre');
    expect(store.status()).toBe('pending');
    expect(store.hasUnpublishedChanges()).toBe(true);
    http.expectNone(`${PAGE_URL}/draft`);

    await autosave();
    const request = http.expectOne(`${PAGE_URL}/draft`);
    expect(request.request.method).toBe('PUT');
    expect(request.request.body.draft_version).toBe(4);
    expect(request.request.body.content.blocks[0].title.fr).toBe('Nouveau titre');
    expect(store.status()).toBe('saving');
    request.flush({ draft_version: 5 });
    await vi.runAllTimersAsync();
    expect(store.status()).toBe('saved');

    changeTitle('Encore');
    await autosave();
    const next = http.expectOne(`${PAGE_URL}/draft`);
    expect(next.request.body.draft_version).toBe(5);
    next.flush({ draft_version: 6 });
  });

  it('F023-S04 several quick changes make a single save', async () => {
    changeTitle('A');
    await vi.advanceTimersByTimeAsync(AUTOSAVE_DELAY_MS / 2);
    changeTitle('AB');
    await autosave();
    const request = http.expectOne(`${PAGE_URL}/draft`);
    expect(request.request.body.content.blocks[0].title.fr).toBe('AB');
    request.flush({ draft_version: 5 });
  });

  it('F023-S07 typing the English text keeps the French one', async () => {
    store.lang.set('en');
    const hero = store.content()!.blocks[0] as DraftHeroBlock;
    store.updateBlock({ ...hero, title: { ...hero.title, en: 'The Little Barn' } });
    expect((store.content()!.blocks[0] as DraftHeroBlock).title).toEqual({
      fr: 'La Petite Grange',
      en: 'The Little Barn',
    });
    await autosave();
    http.expectOne(`${PAGE_URL}/draft`).flush({ draft_version: 5 });
  });

  it('F023-S10 a slug already used is reported on the slug, the changes are kept', async () => {
    store.updateSettings({ ...store.content()!.settings, slug: { fr: 'le-gite', en: 'le-gite' } });
    changeTitle('Titre gardé');
    await autosave();
    http
      .expectOne(`${PAGE_URL}/draft`)
      .flush(
        { error: { code: 'slug_taken', message: 'adresse déjà utilisée en fr : « le-gite »' } },
        { status: 409, statusText: 'Conflict' },
      );
    await vi.runAllTimersAsync();
    expect(store.status()).toBe('invalid');
    expect(store.problems().slug).toContain('le-gite');
    expect((store.content()!.blocks[0] as DraftHeroBlock).title.fr).toBe('Titre gardé');
  });

  it('F023-S11 a draft changed elsewhere stops autosave until the latest draft is loaded', async () => {
    changeTitle('Mon titre');
    await autosave();
    http
      .expectOne(`${PAGE_URL}/draft`)
      .flush(
        { error: { code: 'version_conflict', message: 'brouillon modifié' } },
        { status: 409, statusText: 'Conflict' },
      );
    await vi.runAllTimersAsync();
    expect(store.status()).toBe('conflict');

    changeTitle('Autre');
    await autosave();
    http.expectNone(`${PAGE_URL}/draft`);

    const reloading = store.reloadAfterConflict();
    http.expectOne(PAGE_URL).flush(adminPage(7));
    await reloading;
    expect(store.status()).toBe('saved');
    expect(store.lostChanges()).toEqual(['Bandeau']);
    expect((store.content()!.blocks[0] as DraftHeroBlock).title.fr).toBe('La Petite Grange');
  });

  it('F023-S12 publishes the draft version that was summarised', async () => {
    const asking = store.changes();
    await vi.runAllTimersAsync();
    http.expectOne(`${PAGE_URL}/changes`).flush(CHANGES);
    expect(await asking).toEqual(CHANGES);

    const publishing = store.publish(5);
    const request = http.expectOne(`${PAGE_URL}/publish`);
    expect(request.request.body).toEqual({ draft_version: 5 });
    request.flush(null, { status: 204, statusText: 'No Content' });
    await vi.runAllTimersAsync();
    http.expectOne(PAGE_URL).flush(adminPage(5));
    expect(await publishing).toBe('published');
  });

  it('F023-S12 saves pending changes before asking for the summary', async () => {
    changeTitle('Avant publication');
    const asking = store.changes();
    await vi.runAllTimersAsync();
    http.expectOne(`${PAGE_URL}/draft`).flush({ draft_version: 5 });
    await vi.runAllTimersAsync();
    http.expectOne(`${PAGE_URL}/changes`).flush(CHANGES);
    expect(await asking).toEqual(CHANGES);
  });

  it('F023-S13 publishing a draft that changed meanwhile is refused', async () => {
    const publishing = store.publish(5);
    http
      .expectOne(`${PAGE_URL}/publish`)
      .flush(
        { error: { code: 'version_conflict', message: 'brouillon modifié' } },
        { status: 409, statusText: 'Conflict' },
      );
    expect(await publishing).toBe('changed');
  });

  it('F023-S14 restoring a version loads it into the draft', async () => {
    const restoring = store.restore(1);
    await vi.runAllTimersAsync();
    http.expectOne(`${PAGE_URL}/versions/1/restore`).flush({ draft_version: 5 });
    await vi.runAllTimersAsync();
    const restored = adminPage(5);
    restored.draft = { ...restored.draft, blocks: [] };
    http.expectOne(PAGE_URL).flush(restored);
    expect(await restoring).toBe(true);
    expect(store.content()!.blocks).toEqual([]);
    expect(store.hasUnpublishedChanges()).toBe(true);
  });
});
