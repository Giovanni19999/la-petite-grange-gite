import { HttpClient, provideHttpClient, withInterceptors } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';

import { API_BASE_URL, apiBaseUrlInterceptor } from './api-base-url.interceptor';

function setup(baseUrl?: string): { http: HttpClient; controller: HttpTestingController } {
  TestBed.configureTestingModule({
    providers: [
      provideHttpClient(withInterceptors([apiBaseUrlInterceptor])),
      provideHttpClientTesting(),
      ...(baseUrl ? [{ provide: API_BASE_URL, useValue: baseUrl }] : []),
    ],
  });
  return { http: TestBed.inject(HttpClient), controller: TestBed.inject(HttpTestingController) };
}

describe('apiBaseUrlInterceptor', () => {
  it('keeps relative API URLs in the browser', () => {
    const { http, controller } = setup();
    http.get('/api/pages/menu').subscribe();
    controller.expectOne('/api/pages/menu').flush([]);
  });

  it('uses the internal API URL on the server', () => {
    const { http, controller } = setup('http://api:8080/');
    http.get('/api/pages/menu').subscribe();
    controller.expectOne('http://api:8080/api/pages/menu').flush([]);
  });
});
