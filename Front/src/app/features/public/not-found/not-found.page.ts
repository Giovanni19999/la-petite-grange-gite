import { ChangeDetectionStrategy, Component } from '@angular/core';
import { RouterLink } from '@angular/router';

/** Shown for any unknown URL (served with HTTP 404 by the server). */
@Component({
  selector: 'app-not-found-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink],
  template: `
    <section class="mx-auto flex max-w-xl flex-col items-center gap-4 px-4 py-16 text-center">
      <h1 class="text-2xl font-semibold">Page introuvable</h1>
      <p class="text-muted-foreground">Cette page n'existe pas ou a été déplacée.</p>
      <a routerLink="/" class="font-medium text-primary underline underline-offset-4">
        Retour à l'accueil
      </a>
    </section>
  `,
})
export class NotFoundPage {}
