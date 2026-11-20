import { DOCUMENT } from '@angular/common';
import { ChangeDetectionStrategy, Component, effect, inject, input } from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';

import { NotFoundPage } from '../not-found/not-found.page';
import { CurrentPage } from './data-access/current-page';
import { Lang, PublicPage } from './data-access/pages.models';
import { BlockRenderer } from './ui/blocks/block-renderer';

const SITE_NAME = 'La Petite Grange';

/** A published page (F002): its blocks, title, description and language alternates. */
@Component({
  selector: 'app-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [BlockRenderer, NotFoundPage],
  template: `
    @if (page(); as p) {
      <article>
        @if (p.blocks[0]?.type !== 'hero') {
          <h1 class="mx-auto max-w-5xl px-4 pt-10 text-3xl font-semibold tracking-tight">
            {{ p.title }}
          </h1>
        }
        @for (block of p.blocks; track block.id) {
          <app-block-renderer [block]="block" />
        }
      </article>
    } @else {
      <app-not-found-page />
    }
  `,
})
export class PageView {
  /** Resolved by `pageResolver`; `null` when the page does not exist. */
  readonly page = input<PublicPage | null>(null);

  private readonly title = inject(Title);
  private readonly meta = inject(Meta);
  private readonly document = inject(DOCUMENT);
  private readonly current = inject(CurrentPage);

  constructor() {
    effect(() => {
      const page = this.page();
      this.current.alternates.set(page?.alternates ?? null);
      if (!page) {
        return;
      }
      this.title.setTitle(page.title === SITE_NAME ? SITE_NAME : `${page.title} - ${SITE_NAME}`);
      this.meta.updateTag({ name: 'description', content: page.meta_description });
      this.document.documentElement.lang = page.lang;
      this.setAlternates(page.alternates);
    });
  }

  /** `<link rel="alternate" hreflang>` for search engines. */
  private setAlternates(alternates: Record<Lang, string>): void {
    const head = this.document.head;
    head.querySelectorAll('link[rel="alternate"][hreflang]').forEach((link) => link.remove());
    for (const lang of ['fr', 'en'] as const) {
      const link = this.document.createElement('link');
      link.rel = 'alternate';
      link.hreflang = lang;
      link.href = alternates[lang];
      head.appendChild(link);
    }
  }
}
