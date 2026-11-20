import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { Component } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter, Router } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { authGuard, safeReturnUrl } from './auth.guard';

@Component({ template: '<h1>Tableau de bord</h1>' })
class FakeDashboard {}

@Component({ template: '<h1>Connexion</h1>' })
class FakeLogin {}

/** Lets pending promises and the router run. */
function macrotask(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve));
}

describe('F001 auth guard', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([
          { path: 'admin/connexion', component: FakeLogin },
          { path: 'admin', canActivate: [authGuard], component: FakeDashboard },
        ]),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('F001-S07 sends a visitor without session to the login page, remembering the target', async () => {
    const navigation = harness.navigateByUrl('/admin');
    await macrotask(); // the guard calls the API after the navigation has started
    http
      .expectOne('/api/auth/me')
      .flush(
        { error: { code: 'unauthorized', message: 'Connexion requise' } },
        { status: 401, statusText: 'Unauthorized' },
      );
    await navigation;

    expect(TestBed.inject(Router).url).toBe('/admin/connexion?retour=%2Fadmin');
    expect(harness.routeNativeElement?.textContent).toContain('Connexion');
  });

  it('lets a logged-in owner open the admin', async () => {
    const navigation = harness.navigateByUrl('/admin');
    await macrotask(); // the guard calls the API after the navigation has started
    http.expectOne('/api/auth/me').flush({ email: 'proprio@example.test' });
    await navigation;

    expect(TestBed.inject(Router).url).toBe('/admin');
    expect(harness.routeNativeElement?.textContent).toContain('Tableau de bord');
  });
});

describe('safeReturnUrl', () => {
  it('accepts admin pages only', () => {
    expect(safeReturnUrl('/admin/tarifs')).toBe('/admin/tarifs');
    expect(safeReturnUrl('https://evil.example')).toBe('/admin');
    expect(safeReturnUrl('//evil.example/admin')).toBe('/admin');
    expect(safeReturnUrl('/admin/connexion')).toBe('/admin');
    expect(safeReturnUrl(undefined)).toBe('/admin');
  });
});
