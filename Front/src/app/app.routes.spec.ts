import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { routes } from './app.routes';

describe('F021 routes', () => {
  let harness: RouterTestingHarness;

  beforeEach(async () => {
    TestBed.configureTestingModule({ providers: [provideRouter(routes)] });
    harness = await RouterTestingHarness.create();
  });

  it('shows the site name on the home page', async () => {
    await harness.navigateByUrl('/');

    expect(harness.routeNativeElement?.querySelector('h1')?.textContent).toContain(
      'La Petite Grange',
    );
  });

  it('F021-S03 opens the admin area on /admin', async () => {
    await harness.navigateByUrl('/admin');

    expect(harness.routeNativeElement?.textContent).toContain('Espace administrateur');
  });

  it('F021-S04 shows a not-found page with a link home for unknown URLs', async () => {
    await harness.navigateByUrl('/does-not-exist');

    const page = harness.routeNativeElement;
    expect(page?.querySelector('h1')?.textContent).toContain('Page introuvable');
    expect(page?.querySelector('a')?.getAttribute('href')).toBe('/');
  });
});
