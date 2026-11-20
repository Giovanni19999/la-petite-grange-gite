import { ChangeDetectionStrategy, Component } from '@angular/core';

/** Admin home placeholder, replaced by the real dashboard in later features. */
@Component({
  selector: 'app-admin-dashboard-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <section class="mx-auto max-w-3xl px-4 py-10">
      <h1 class="text-2xl font-semibold">Espace administrateur</h1>
      <p class="mt-2 text-muted-foreground">En construction.</p>
    </section>
  `,
})
export class DashboardPage {}
