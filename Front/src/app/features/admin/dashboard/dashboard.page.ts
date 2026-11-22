import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { firstValueFrom } from 'rxjs';

import { LOGIN_PATH } from '../../../core/auth/auth.guard';
import { AuthStore } from '../../../core/auth/auth.store';
import { MessagingApi } from '../messaging/data-access/messaging.api';

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
        <a hlmBtn variant="outline" routerLink="/admin/reservations">Réservations</a>
        <a hlmBtn variant="outline" routerLink="/admin/pages">Pages du site</a>
        <a hlmBtn variant="outline" routerLink="/admin/medias">Médiathèque</a>
        <a hlmBtn variant="outline" routerLink="/admin/fermetures">Fermetures</a>
        <a hlmBtn variant="outline" routerLink="/admin/tarifs">Tarifs</a>
        <a hlmBtn variant="outline" routerLink="/admin/promotions">Promotions et options</a>
        <a hlmBtn variant="outline" routerLink="/admin/regles">Règles du gîte</a>
        <a hlmBtn variant="outline" routerLink="/admin/modeles">Modèles de messages</a>
        <a hlmBtn variant="outline" routerLink="/admin/messages">Envoi des messages</a>
      </nav>
      @if (messageProblems() > 0) {
        <p class="mt-6 rounded-lg border border-destructive p-3 text-sm" role="alert">
          {{ messageProblems() }}
          {{
            messageProblems() === 1
              ? "message n'a pas pu être envoyé"
              : "messages n'ont pas pu être envoyés"
          }}.
          <a routerLink="/admin/messages" class="underline">Voir l'envoi des messages</a>
        </p>
      }
    </section>
  `,
})
export class DashboardPage {
  protected readonly store = inject(AuthStore);
  private readonly router = inject(Router);
  private readonly messaging = inject(MessagingApi);
  /** Failed or to-check messages (F025). */
  protected readonly messageProblems = signal(0);

  constructor() {
    void this.loadProblems();
  }

  private async loadProblems(): Promise<void> {
    try {
      this.messageProblems.set((await firstValueFrom(this.messaging.problems())).count);
    } catch {
      this.messageProblems.set(0);
    }
  }

  protected async logout(): Promise<void> {
    await this.store.logout();
    await this.router.navigateByUrl(LOGIN_PATH);
  }
}
