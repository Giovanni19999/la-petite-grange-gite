import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Observable } from 'rxjs';

/** Owner returned by the API. */
export interface AdminUser {
  email: string;
}

/** HTTP calls of the authentication API (F001). The session lives in an HttpOnly cookie. */
@Injectable({ providedIn: 'root' })
export class AuthApi {
  private readonly http = inject(HttpClient);

  login(email: string, password: string): Observable<AdminUser> {
    return this.http.post<AdminUser>('/api/auth/login', { email, password });
  }

  logout(): Observable<void> {
    return this.http.post<void>('/api/auth/logout', null);
  }

  me(): Observable<AdminUser> {
    return this.http.get<AdminUser>('/api/auth/me');
  }
}
