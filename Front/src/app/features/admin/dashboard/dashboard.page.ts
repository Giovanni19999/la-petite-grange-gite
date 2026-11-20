import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';

import { LOGIN_PATH } from '../../../core/auth/auth.guard';
import { AuthStore } from '../../../core/auth/auth.store';

/** Admin home: entry points to the admin features (the full dashboard comes later). */
@Component({
  selector: 'app-admin-dashboard-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink, HlmButtonImports],
  template: `
    <section class="mx-auto max-w-3xl px-4 py-10">
      <div class="flex items-center justify-between gap-4">
        <h1 class="text-2xl font-semibold">Espace administrateur</h1>
        <button hlmBtn variant="outline" type="button" (click)="logout()">Se déconnecter</button>
      </div>
      <p class="mt-2 text-muted-foreground">Connecté en tant que {{ store.user()?.email }}.</p>
      <nav class="mt-8 flex flex-wrap gap-2" aria-label="Administration">
        <a hlmBtn variant="outline" routerLink="/admin/pages">Pages du site</a>
        <a hlmBtn variant="outline" routerLink="/admin/medias">Médiathèque</a>
      </nav>
    </section>
  `,
})
export class DashboardPage {
  protected readonly store = inject(AuthStore);
  private readonly router = inject(Router);

  protected async logout(): Promise<void> {
    await this.store.logout();
    await this.router.navigateByUrl(LOGIN_PATH);
  }
}
