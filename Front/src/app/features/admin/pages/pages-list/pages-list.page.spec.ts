import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ApplicationRef, Component } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { Router, provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { AdminPageSummary } from '../data-access/admin-pages.models';
import { PagesListPage } from './pages-list.page';

@Component({ template: '<p>Éditeur</p>' })
class FakeEditor {}

const PAGES: AdminPageSummary[] = [
  {
    id: 1,
    is_home: true,
    title: 'La Petite Grange',
    slug: 'accueil',
    draft_version: 3,
    published_at: '2026-11-20T10:00:00Z',
    unpublished_changes: false,
  },
  {
    id: 2,
    is_home: false,
    title: 'Le gîte',
    slug: 'le-gite',
    draft_version: 4,
    published_at: '2026-11-20T10:00:00Z',
    unpublished_changes: true,
  },
];

describe('F023 pages list', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([
          { path: 'admin/pages', component: PagesListPage },
          { path: 'admin/pages/:id', component: FakeEditor },
        ]),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
    await harness.navigateByUrl('/admin/pages');
    http.expectOne('/api/admin/pages').flush(PAGES);
    await settle();
  });

  afterEach(() => http.verify());

  const page = () => harness.routeNativeElement as HTMLElement;
  const rows = () => Array.from(page().querySelectorAll('[data-testid="page-row"]'));

  it('F023-S01 lists the pages with title, slug, home and not published markers', () => {
    expect(rows().map((row) => row.querySelector('a')?.textContent?.trim())).toEqual([
      'La Petite Grange',
      'Le gîte',
    ]);
    expect(rows()[0].textContent).toContain('Accueil');
    expect(rows()[1].textContent).toContain('/le-gite');
    expect(rows()[0].querySelector('[data-testid="not-published"]')).toBeNull();
    expect(rows()[1].querySelector('[data-testid="not-published"]')?.textContent).toContain(
      'Modifications non publiées',
    );
  });

  it('F023-S02 creates a page and opens it in the editor; the home page cannot be deleted', async () => {
    expect(rows()[0].textContent).not.toContain('Supprimer');
    expect(rows()[1].textContent).toContain('Supprimer');

    const newPage = Array.from(page().querySelectorAll('button')).find((button) =>
      button.textContent?.includes('Nouvelle page'),
    )!;
    newPage.click();
    await settle();
    const [title] = Array.from(page().querySelectorAll<HTMLInputElement>('form input'));
    title.value = 'Activités';
    title.dispatchEvent(new Event('input'));
    await settle();
    expect(page().querySelectorAll<HTMLInputElement>('form input')[1].value).toBe('activites');

    page().querySelector('form')!.dispatchEvent(new Event('submit'));
    const request = http.expectOne('/api/admin/pages');
    expect(request.request.method).toBe('POST');
    expect(request.request.body.content.settings.slug).toEqual({
      fr: 'activites',
      en: 'activites',
    });
    expect(request.request.body.content.settings.title.fr).toBe('Activités');
    request.flush({ id: 9 }, { status: 201, statusText: 'Created' });
    await settle();

    expect(TestBed.inject(Router).url).toBe('/admin/pages/9');
  });

  it('F023-S02 deletes a page after confirmation', async () => {
    const remove = Array.from(rows()[1].querySelectorAll('button')).find((button) =>
      button.textContent?.includes('Supprimer'),
    )!;
    remove.click();
    await settle();
    expect(document.body.textContent).toContain('Supprimer la page « Le gîte » ?');
    (document.querySelector('[data-slot="alert-dialog-action"]') as HTMLButtonElement).click();
    const request = http.expectOne('/api/admin/pages/2');
    expect(request.request.method).toBe('DELETE');
    request.flush(null, { status: 204, statusText: 'No Content' });
    await settle();
    http.expectOne('/api/admin/pages').flush([PAGES[0]]);
    await settle();
    expect(rows()).toHaveLength(1);
  });
});

/** Lets pending promises (HTTP responses turned into promises) finish, then renders. */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve));
  await TestBed.inject(ApplicationRef).whenStable();
}
