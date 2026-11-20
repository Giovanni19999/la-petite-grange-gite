import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { Component } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter, Router, withComponentInputBinding } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';

import { LoginPage } from './login.page';

@Component({ template: '<h1>Tarifs</h1>' })
class FakeTarifs {}

describe('F001 login page', () => {
  let harness: RouterTestingHarness;
  let http: HttpTestingController;

  beforeEach(async () => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter(
          [
            { path: 'admin/connexion', component: LoginPage },
            { path: 'admin/tarifs', component: FakeTarifs },
          ],
          withComponentInputBinding(),
        ),
      ],
    });
    harness = await RouterTestingHarness.create();
    http = TestBed.inject(HttpTestingController);
    await harness.navigateByUrl('/admin/connexion?retour=%2Fadmin%2Ftarifs');
  });

  afterEach(() => http.verify());

  function fillAndSubmit(email: string, password: string): void {
    const page = harness.routeNativeElement as HTMLElement;
    const [emailInput, passwordInput] = Array.from(page.querySelectorAll('input'));
    emailInput.value = email;
    emailInput.dispatchEvent(new Event('input'));
    passwordInput.value = password;
    passwordInput.dispatchEvent(new Event('input'));
    page.querySelector('form')?.dispatchEvent(new Event('submit'));
  }

  it('F001-S07 goes back to the requested admin page after a successful login', async () => {
    fillAndSubmit('proprio@example.test', 'correct horse battery');
    const request = http.expectOne('/api/auth/login');
    expect(request.request.body).toEqual({
      email: 'proprio@example.test',
      password: 'correct horse battery',
    });
    request.flush({ email: 'proprio@example.test' });
    await harness.fixture.whenStable();

    expect(TestBed.inject(Router).url).toBe('/admin/tarifs');
  });

  it('F001-S02 shows the generic error message on wrong credentials', async () => {
    fillAndSubmit('proprio@example.test', 'wrong password!');
    http
      .expectOne('/api/auth/login')
      .flush(
        { error: { code: 'invalid_credentials', message: 'Email ou mot de passe incorrect' } },
        { status: 401, statusText: 'Unauthorized' },
      );
    await harness.fixture.whenStable();
    harness.detectChanges();

    expect(harness.routeNativeElement?.querySelector('[role=alert]')?.textContent).toContain(
      'Email ou mot de passe incorrect.',
    );
    expect(TestBed.inject(Router).url).toContain('/admin/connexion');
  });

  it('F001-S04 explains a temporary block', async () => {
    fillAndSubmit('proprio@example.test', 'correct horse battery');
    http
      .expectOne('/api/auth/login')
      .flush(
        { error: { code: 'too_many_attempts', message: 'Trop de tentatives' } },
        { status: 429, statusText: 'Too Many Requests' },
      );
    await harness.fixture.whenStable();
    harness.detectChanges();

    expect(harness.routeNativeElement?.querySelector('[role=alert]')?.textContent).toContain(
      'Trop de tentatives',
    );
  });
});
