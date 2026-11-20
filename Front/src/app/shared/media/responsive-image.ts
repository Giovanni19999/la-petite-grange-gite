import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';

import { PublicImage } from '../../features/public/pages/data-access/pages.models';

/**
 * An image of the media library (F003): WebP `srcset` with the real `sizes`, intrinsic width
 * and height (no layout shift), lazy loading unless it is the first image of the page, and
 * the focal point as `object-position` so that cropping keeps the important part visible.
 */
@Component({
  selector: 'app-responsive-image',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'contents' },
  template: `
    @let img = image();
    <img
      [src]="src()"
      [attr.srcset]="srcset()"
      [attr.sizes]="sizes()"
      [width]="img.width"
      [height]="img.height"
      [alt]="img.alt"
      [attr.loading]="priority() ? 'eager' : 'lazy'"
      [attr.fetchpriority]="priority() ? 'high' : null"
      decoding="async"
      [class]="imageClass()"
      [style.object-position]="img.focal_x + '% ' + img.focal_y + '%'"
    />
  `,
})
export class ResponsiveImage {
  readonly image = input.required<PublicImage>();
  /** Width of the image on screen, for the browser to pick the right file. */
  readonly sizes = input('100vw');
  /** The first image of the page: loaded at once, with high priority. */
  readonly priority = input(false);
  /** Classes of the `<img>` (e.g. `h-full w-full object-cover`). */
  readonly imageClass = input('h-auto w-full');

  protected readonly srcset = computed(() =>
    this.image()
      .sources.map((source) => `${source.url} ${source.width}w`)
      .join(', '),
  );
  /** Fallback for browsers without `srcset`: a middle size. */
  protected readonly src = computed(() => {
    const sources = this.image().sources;
    return sources[Math.min(1, sources.length - 1)]?.url ?? '';
  });
}
