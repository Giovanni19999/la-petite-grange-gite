import { ChangeDetectionStrategy, Component, RESPONSE_INIT, inject } from '@angular/core';
import { RouterLink } from '@angular/router';

/** Shown for any unknown URL; sets the HTTP status to 404 when rendered on the server. */
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
export class NotFoundPage {
  constructor() {
    const response = inject(RESPONSE_INIT, { optional: true });
    if (response) {
      response.status = 404;
    }
  }
}
