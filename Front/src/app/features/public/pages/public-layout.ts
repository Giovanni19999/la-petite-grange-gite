import { ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';

import { CurrentPage } from './data-access/current-page';
import { Lang, MenuItem } from './data-access/pages.models';

/** Public pages layout: header with the menu of the current language and the language switch. */
@Component({
  selector: 'app-public-layout',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterOutlet, RouterLink, RouterLinkActive],
  template: `
    <header class="border-b">
      <nav
        class="mx-auto flex max-w-5xl flex-wrap items-center gap-x-6 gap-y-2 px-4 py-3"
        [attr.aria-label]="lang() === 'en' ? 'Main menu' : 'Menu principal'"
      >
        <a [routerLink]="homePath()" class="mr-auto font-semibold">La Petite Grange</a>
        @for (item of menu(); track item.path) {
          @if (item.path !== homePath()) {
            <a
              class="text-sm text-muted-foreground hover:text-foreground"
              [routerLink]="item.path"
              routerLinkActive="text-foreground font-medium"
              >{{ item.title }}</a
            >
          }
        }
        <a
          class="rounded-md border px-2 py-1 text-xs font-medium"
          [routerLink]="otherLanguagePath()"
          [attr.hreflang]="otherLang()"
          [attr.lang]="otherLang()"
          >{{ otherLang() === 'en' ? 'EN' : 'FR' }}</a
        >
      </nav>
    </header>
    <main class="@container">
      <router-outlet />
    </main>
  `,
})
export class PublicLayout {
  /** From the route data. */
  readonly lang = input<Lang>('fr');
  /** Resolved by `menuResolver`. */
  readonly menu = input<MenuItem[]>([]);

  private readonly current = inject(CurrentPage);

  protected readonly homePath = computed(() => (this.lang() === 'en' ? '/en' : '/'));
  protected readonly otherLang = computed<Lang>(() => (this.lang() === 'en' ? 'fr' : 'en'));
  protected readonly otherLanguagePath = computed(
    () =>
      this.current.alternates()?.[this.otherLang()] ?? (this.otherLang() === 'en' ? '/en' : '/'),
  );
}
