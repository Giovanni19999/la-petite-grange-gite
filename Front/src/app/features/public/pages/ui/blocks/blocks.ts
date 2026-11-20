import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';

import {
  Align,
  ColumnsBlock,
  GalleryBlock,
  HeroBlock,
  TextBlock,
  TextImageBlock,
} from '../../data-access/pages.models';

const TEXT_ALIGN: Record<Align, string> = {
  left: 'text-left',
  center: 'text-center',
  right: 'text-right',
};

const ITEMS_ALIGN: Record<Align, string> = {
  left: 'items-start text-left',
  center: 'items-center text-center',
  right: 'items-end text-right',
};

const HERO_HEIGHT: Record<HeroBlock['height'], string> = {
  small: 'min-h-[30vh]',
  medium: 'min-h-[50vh]',
  large: 'min-h-[70vh]',
};

/** Large banner: title (the page h1), subtitle and call to action. Image comes with F003. */
@Component({
  selector: 'app-hero-block',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div
      class="relative flex flex-col justify-center gap-4 bg-gradient-to-br from-primary to-primary/70 px-4 py-16 text-primary-foreground"
      [class]="heightClass() + ' ' + alignClass()"
    >
      <div class="mx-auto flex w-full max-w-4xl flex-col gap-4" [class]="alignClass()">
        <h1 class="text-4xl font-semibold tracking-tight sm:text-5xl">{{ block().title }}</h1>
        @if (block().subtitle) {
          <p class="text-lg opacity-90 sm:text-xl">{{ block().subtitle }}</p>
        }
        @if (block().button_label && block().button_link) {
          <a
            class="mt-2 inline-flex w-fit rounded-md bg-background px-6 py-3 font-medium text-foreground shadow-sm transition hover:opacity-90"
            [href]="block().button_link"
          >
            {{ block().button_label }}
          </a>
        }
      </div>
    </div>
  `,
})
export class HeroBlockView {
  readonly block = input.required<HeroBlock>();
  protected readonly heightClass = computed(() => HERO_HEIGHT[this.block().height]);
  protected readonly alignClass = computed(() => ITEMS_ALIGN[this.block().align]);
}

/** Rich text. */
@Component({
  selector: 'app-text-block',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div
      class="rich-text mx-auto px-4"
      [class]="(block().width === 'wide' ? 'max-w-5xl ' : 'max-w-2xl ') + alignClass()"
      [innerHTML]="block().body"
    ></div>
  `,
})
export class TextBlockView {
  readonly block = input.required<TextBlock>();
  protected readonly alignClass = computed(() => TEXT_ALIGN[this.block().align]);
}

/** Rich text next to an image (left or right). Without image, the text takes the width. */
@Component({
  selector: 'app-text-image-block',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="mx-auto grid max-w-5xl gap-8 px-4" [class]="gridClass()">
      <div class="rich-text" [innerHTML]="block().body"></div>
      @if (block().image !== null) {
        <div
          class="aspect-[4/3] rounded-lg bg-muted"
          [class.md:order-first]="block().image_position === 'left'"
          role="img"
          aria-label=""
        ></div>
      }
    </div>
  `,
})
export class TextImageBlockView {
  readonly block = input.required<TextImageBlock>();
  protected readonly gridClass = computed(() => {
    if (this.block().image === null) {
      return 'max-w-2xl';
    }
    return this.block().ratio === '33-67' ? 'md:grid-cols-[2fr_1fr]' : 'md:grid-cols-2';
  });
}

/** 2 to 4 columns with a title and rich text each. */
@Component({
  selector: 'app-columns-block',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="mx-auto grid max-w-5xl gap-8 px-4" [class]="gridClass()">
      @for (item of block().items; track $index) {
        <div class="flex flex-col gap-2">
          @if (item.title) {
            <h2 class="text-xl font-semibold">{{ item.title }}</h2>
          }
          <div class="rich-text" [innerHTML]="item.body"></div>
        </div>
      }
    </div>
  `,
})
export class ColumnsBlockView {
  readonly block = input.required<ColumnsBlock>();
  protected readonly gridClass = computed(
    () =>
      ({ 2: 'sm:grid-cols-2', 3: 'sm:grid-cols-3', 4: 'sm:grid-cols-2 lg:grid-cols-4' })[
        this.block().items.length
      ] ?? 'sm:grid-cols-2',
  );
}

/** Image grid. Images are shown once the media library exists (F003). */
@Component({
  selector: 'app-gallery-block',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (block().images.length > 0) {
      <div class="mx-auto grid max-w-5xl grid-cols-2 gap-2 px-4" [class]="columnsClass()">
        @for (image of block().images; track image) {
          <div class="aspect-square rounded-md bg-muted"></div>
        }
      </div>
    }
  `,
})
export class GalleryBlockView {
  readonly block = input.required<GalleryBlock>();
  protected readonly columnsClass = computed(
    () => ({ 3: 'md:grid-cols-3', 4: 'md:grid-cols-4' })[this.block().columns] ?? '',
  );
}
