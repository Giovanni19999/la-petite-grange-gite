import { HttpErrorResponse } from '@angular/common/http';
import { Injectable, computed, inject, signal } from '@angular/core';
import { firstValueFrom } from 'rxjs';

import { AdminUser, AuthApi } from './auth.api';

/** Why a login failed, as shown to the owner. */
export type LoginFailure = 'invalid_credentials' | 'too_many_attempts' | 'unavailable';

/** Authentication state of the admin (signals store). */
@Injectable({ providedIn: 'root' })
export class AuthStore {
  private readonly api = inject(AuthApi);
  private readonly currentUser = signal<AdminUser | null>(null);
  private sessionChecked: Promise<void> | null = null;

  readonly user = this.currentUser.asReadonly();
  readonly isLoggedIn = computed(() => this.currentUser() !== null);

  /** Asks the API once whether a session exists. */
  ensureSessionChecked(): Promise<void> {
    this.sessionChecked ??= firstValueFrom(this.api.me()).then(
      (user) => this.currentUser.set(user),
      () => this.currentUser.set(null),
    );
    return this.sessionChecked;
  }

  /** Returns `null` on success, otherwise the reason of the failure. */
  async login(email: string, password: string): Promise<LoginFailure | null> {
    try {
      this.currentUser.set(await firstValueFrom(this.api.login(email, password)));
      this.sessionChecked = Promise.resolve();
      return null;
    } catch (error) {
      if (error instanceof HttpErrorResponse && error.status === 401) {
        return 'invalid_credentials';
      }
      if (error instanceof HttpErrorResponse && error.status === 429) {
        return 'too_many_attempts';
      }
      return 'unavailable';
    }
  }

  async logout(): Promise<void> {
    try {
      await firstValueFrom(this.api.logout());
    } finally {
      this.currentUser.set(null);
      this.sessionChecked = Promise.resolve();
    }
  }
}
