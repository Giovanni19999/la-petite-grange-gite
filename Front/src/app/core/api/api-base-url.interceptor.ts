import { HttpInterceptorFn } from '@angular/common/http';
import { InjectionToken, inject } from '@angular/core';

/**
 * Absolute base URL of the API, provided only on the server (ADR-0013): the SSR server
 * cannot use relative URLs. In the browser it is absent and `/api/...` stays relative.
 */
export const API_BASE_URL = new InjectionToken<string>('API_BASE_URL');

/** Prefixes relative `/api/...` requests with {@link API_BASE_URL} when it is provided. */
export const apiBaseUrlInterceptor: HttpInterceptorFn = (request, next) => {
  const baseUrl = inject(API_BASE_URL, { optional: true });
  if (baseUrl && request.url.startsWith('/api/')) {
    return next(request.clone({ url: baseUrl.replace(/\/$/, '') + request.url }));
  }
  return next(request);
};
