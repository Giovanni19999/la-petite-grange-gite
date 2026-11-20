import { ChangeDetectionStrategy, Component } from '@angular/core';

/** Public home page. Real content comes from the page editor (F002). */
@Component({
  selector: 'app-home-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <section class="mx-auto flex max-w-3xl flex-col items-center gap-6 px-4 py-16 text-center">
      <h1 class="text-3xl font-semibold tracking-tight sm:text-4xl">La Petite Grange</h1>
      <p class="text-muted-foreground">Gîte classé 3 étoiles</p>
      <a
        href="#reserver"
        class="rounded-md bg-primary px-6 py-3 font-medium text-primary-foreground shadow-sm transition hover:opacity-90 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
      >
        Réserver
      </a>
    </section>
  `,
})
export class HomePage {}
