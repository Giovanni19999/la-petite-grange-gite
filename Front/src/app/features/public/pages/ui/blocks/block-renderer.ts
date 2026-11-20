import { ChangeDetectionStrategy, Component, input } from '@angular/core';

import { PublicBlock } from '../../data-access/pages.models';
import { BlockFrame } from './block-frame';
import {
  ColumnsBlockView,
  GalleryBlockView,
  HeroBlockView,
  TextBlockView,
  TextImageBlockView,
} from './blocks';

/**
 * Block registry (ADR-0009): picks the component of each block type. A new block type is a
 * new component plus one case here; the page component does not change.
 */
@Component({
  selector: 'app-block-renderer',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BlockFrame,
    HeroBlockView,
    TextBlockView,
    TextImageBlockView,
    ColumnsBlockView,
    GalleryBlockView,
  ],
  template: `
    @let b = block();
    @if (b.type === 'hero') {
      <app-hero-block [block]="b" [class]="b.hide_on_mobile ? '@max-3xl:hidden' : ''" />
    } @else {
      <app-block-frame
        [background]="b.background"
        [spacing]="b.spacing"
        [hideOnMobile]="b.hide_on_mobile"
      >
        @switch (b.type) {
          @case ('text') {
            <app-text-block [block]="b" />
          }
          @case ('text_image') {
            <app-text-image-block [block]="b" />
          }
          @case ('columns') {
            <app-columns-block [block]="b" />
          }
          @case ('gallery') {
            <app-gallery-block [block]="b" />
          }
        }
      </app-block-frame>
    }
  `,
})
export class BlockRenderer {
  readonly block = input.required<PublicBlock>();
}
