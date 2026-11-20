import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { provideRouter, withComponentInputBinding, withRouterConfig } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { routes } from './app.routes';
import { PublicPage } from './features/public/pages/data-access/pages.models';

const HOME_FR: PublicPage = {
  lang: 'fr',
  title: 'La Petite Grange',
  meta_description: 'Gîte classé 3 étoiles',
  alternates: { fr: '/', en: '/en' },
  blocks: [
    {
      id: 'hero',
      type: 'hero',
      image: null,
      title: 'La Petite Grange',
      subtitle: 'Gîte classé 3 étoiles',
      button_label: 'Réserver',
      button_link: '#reserver',
      align: 'center',
      height: 'large',
      darken: 30,
      background: 'background',
      spacing: 'medium',
      hide_on_mobile: false,
    },
    {
      id: 'intro',
      type: 'text',
      body: '<p>Bienvenue</p>',
      width: 'narrow',
      align: 'center',
      background: 'muted',
      spacing: 'large',
      hide_on_mobile: false,
    },
  ],
};

/** Lets pending promises, resolvers and HTTP calls run. */
function macrotask(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve));
}

describe('F002 / F021 public routes', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter(
          routes,
          withComponentInputBinding(),
          withRouterConfig({ paramsInheritanceStrategy: 'always' }),
        ),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  async function navigate(url: string, respond: () => void): Promise<HTMLElement> {
    const navigation = harness.navigateByUrl(url);
    await macrotask();
    respond();
    await navigation;
    harness.detectChanges();
    return harness.fixture.nativeElement as HTMLElement;
  }

  it('F002-S01 renders the French home page blocks with the menu', async () => {
    const page = await navigate('/', () => {
      http.expectOne('/api/pages/menu?lang=fr').flush([
        { title: 'La Petite Grange', path: '/' },
        { title: 'Le gîte', path: '/le-gite' },
      ]);
      http.expectOne('/api/pages/home?lang=fr').flush(HOME_FR);
    });

    expect(page.querySelector('h1')?.textContent).toContain('La Petite Grange');
    expect(page.querySelector('.rich-text')?.innerHTML).toContain('Bienvenue');
    expect(page.querySelector('nav')?.textContent).toContain('Le gîte');
    expect(page.querySelector('a[hreflang="en"]')?.getAttribute('href')).toBe('/en');
  });

  it('F002-S01 asks the English page by its English slug', async () => {
    const page = await navigate('/en/the-cottage', () => {
      http.expectOne('/api/pages/menu?lang=en').flush([]);
      http.expectOne('/api/pages/by-slug/the-cottage?lang=en').flush({
        ...HOME_FR,
        lang: 'en',
        title: 'The cottage',
        alternates: { fr: '/le-gite', en: '/en/the-cottage' },
        blocks: [],
      });
    });

    expect(page.querySelector('h1')?.textContent).toContain('The cottage');
    expect(page.querySelector('a[hreflang="fr"]')?.getAttribute('href')).toBe('/le-gite');
  });

  it('F002-S03 shows the not-found page for an unknown slug', async () => {
    const page = await navigate('/page-inconnue', () => {
      http.expectOne('/api/pages/menu?lang=fr').flush([]);
      http
        .expectOne('/api/pages/by-slug/page-inconnue?lang=fr')
        .flush({ error: { code: 'not_found' } }, { status: 404, statusText: 'Not Found' });
    });

    expect(page.querySelector('h1')?.textContent).toContain('Page introuvable');
  });

  it('F021-S03 keeps the admin area separate', async () => {
    const navigation = harness.navigateByUrl('/admin');
    // The admin area is loaded lazily: wait until its guard calls the API.
    let requests = http.match('/api/auth/me');
    for (let attempt = 0; requests.length === 0 && attempt < 20; attempt++) {
      await macrotask();
      requests = http.match('/api/auth/me');
    }
    requests[0].flush(
      { error: { code: 'unauthorized' } },
      { status: 401, statusText: 'Unauthorized' },
    );
    await navigation;
    harness.detectChanges();
    const page = harness.fixture.nativeElement as HTMLElement;

    expect(page.textContent).toContain('Espace administrateur');
  });

  it('F021-S04 shows the not-found page for unknown deep URLs', async () => {
    await harness.navigateByUrl('/a/b/c');

    expect(harness.routeNativeElement?.querySelector('h1')?.textContent).toContain(
      'Page introuvable',
    );
  });
});
