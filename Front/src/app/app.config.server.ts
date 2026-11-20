import { mergeApplicationConfig, ApplicationConfig } from '@angular/core';
import { provideServerRendering, withRoutes } from '@angular/ssr';
import { appConfig } from './app.config';
import { API_BASE_URL } from './core/api/api-base-url.interceptor';
import { serverRoutes } from './app.routes.server';

const serverConfig: ApplicationConfig = {
  providers: [
    provideServerRendering(withRoutes(serverRoutes)),
    // The SSR server reaches the API directly (ADR-0013).
    { provide: API_BASE_URL, useValue: process.env['API_INTERNAL_URL'] ?? 'http://127.0.0.1:8080' },
  ],
};

export const config = mergeApplicationConfig(appConfig, serverConfig);
