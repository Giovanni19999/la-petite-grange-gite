import { ChangeDetectionStrategy, Component, inject, input, signal } from '@angular/core';
import { NonNullableFormBuilder, ReactiveFormsModule, Validators } from '@angular/forms';
import { Router } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { HlmInputImports } from '@spartan-ng/helm/input';
import { HlmLabelImports } from '@spartan-ng/helm/label';

import { safeReturnUrl } from '../../../core/auth/auth.guard';
import { AuthStore, LoginFailure } from '../../../core/auth/auth.store';

const MESSAGES: Record<LoginFailure, string> = {
  invalid_credentials: 'Email ou mot de passe incorrect.',
  too_many_attempts: 'Trop de tentatives de connexion, réessayez dans quelques minutes.',
  unavailable: 'Connexion impossible pour le moment, réessayez plus tard.',
};

/** Admin login page (F001). */
@Component({
  selector: 'app-login-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ReactiveFormsModule, HlmButtonImports, HlmInputImports, HlmLabelImports],
  template: `
    <section class="mx-auto max-w-sm px-4 py-12">
      <h1 class="text-2xl font-semibold">Espace administrateur</h1>
      <p class="mt-1 text-muted-foreground">Connectez-vous pour gérer le site.</p>

      <form class="mt-8 flex flex-col gap-4" [formGroup]="form" (ngSubmit)="submit()">
        <label hlmLabel class="flex flex-col items-start gap-2">
          Email
          <input
            hlmInput
            class="w-full"
            type="email"
            formControlName="email"
            autocomplete="username"
            inputmode="email"
            required
          />
        </label>
        <label hlmLabel class="flex flex-col items-start gap-2">
          Mot de passe
          <input
            hlmInput
            class="w-full"
            type="password"
            formControlName="password"
            autocomplete="current-password"
            required
          />
        </label>

        @if (error(); as message) {
          <p class="text-sm text-destructive" role="alert">{{ message }}</p>
        }

        <button hlmBtn size="lg" type="submit" [disabled]="pending()">
          {{ pending() ? 'Connexion…' : 'Se connecter' }}
        </button>
      </form>
    </section>
  `,
})
export class LoginPage {
  private readonly store = inject(AuthStore);
  private readonly router = inject(Router);

  /** Page the owner wanted before being sent to the login page (query parameter). */
  readonly retour = input<string>();

  protected readonly form = inject(NonNullableFormBuilder).group({
    email: ['', [Validators.required, Validators.email]],
    password: ['', Validators.required],
  });
  protected readonly pending = signal(false);
  protected readonly error = signal<string | null>(null);

  protected async submit(): Promise<void> {
    if (this.form.invalid) {
      this.form.markAllAsTouched();
      this.error.set('Renseignez votre email et votre mot de passe.');
      return;
    }
    this.pending.set(true);
    this.error.set(null);
    const { email, password } = this.form.getRawValue();
    const failure = await this.store.login(email, password);
    this.pending.set(false);
    if (failure) {
      this.error.set(MESSAGES[failure]);
      this.form.controls.password.reset();
      return;
    }
    await this.router.navigateByUrl(safeReturnUrl(this.retour()));
  }
}
