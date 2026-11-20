import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';

import {
  Align,
  ColumnsBlock,
  GalleryBlock,
  HeroBlock,
  PublicBlock,
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
        <h1
          class="text-4xl font-semibold tracking-tight @2xl:text-5xl"
          [class.opacity-60]="isFallback(block(), 'title')"
        >
          {{ block().title }}
        </h1>
        @if (block().subtitle) {
          <p
            class="text-lg opacity-90 @2xl:text-xl"
            [class.opacity-60]="isFallback(block(), 'subtitle')"
          >
            {{ block().subtitle }}
          </p>
        }
        @if (block().button_label && block().button_link) {
          <a
            class="mt-2 inline-flex w-fit rounded-md bg-background px-6 py-3 font-medium text-foreground shadow-sm transition hover:opacity-90"
            [href]="block().button_link"
            [class.opacity-60]="isFallback(block(), 'button_label')"
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
  protected readonly isFallback = isFallback;
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
      [class.opacity-60]="isFallback(block(), 'body')"
      [innerHTML]="block().body"
    ></div>
  `,
})
export class TextBlockView {
  readonly block = input.required<TextBlock>();
  protected readonly isFallback = isFallback;
  protected readonly alignClass = computed(() => TEXT_ALIGN[this.block().align]);
}

/** Rich text next to an image (left or right). Without image, the text takes the width. */
@Component({
  selector: 'app-text-image-block',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="mx-auto grid max-w-5xl gap-8 px-4" [class]="gridClass()">
      <div
        class="rich-text"
        [class.opacity-60]="isFallback(block(), 'body')"
        [innerHTML]="block().body"
      ></div>
      @if (block().image !== null) {
        <div
          class="aspect-[4/3] rounded-lg bg-muted"
          [class]="block().image_position === 'left' ? '@3xl:order-first' : ''"
          role="img"
          aria-label=""
        ></div>
      }
    </div>
  `,
})
export class TextImageBlockView {
  readonly block = input.required<TextImageBlock>();
  protected readonly isFallback = isFallback;
  protected readonly gridClass = computed(() => {
    if (this.block().image === null) {
      return 'max-w-2xl';
    }
    return this.block().ratio === '33-67' ? '@3xl:grid-cols-[2fr_1fr]' : '@3xl:grid-cols-2';
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
            <h2
              class="text-xl font-semibold"
              [class.opacity-60]="isFallback(block(), 'items.' + $index + '.title')"
            >
              {{ item.title }}
            </h2>
          }
          <div
            class="rich-text"
            [class.opacity-60]="isFallback(block(), 'items.' + $index + '.body')"
            [innerHTML]="item.body"
          ></div>
        </div>
      }
    </div>
  `,
})
export class ColumnsBlockView {
  readonly block = input.required<ColumnsBlock>();
  protected readonly isFallback = isFallback;
  protected readonly gridClass = computed(
    () =>
      ({ 2: '@2xl:grid-cols-2', 3: '@2xl:grid-cols-3', 4: '@2xl:grid-cols-2 @5xl:grid-cols-4' })[
        this.block().items.length
      ] ?? '@2xl:grid-cols-2',
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
    () => ({ 3: '@3xl:grid-cols-3', 4: '@3xl:grid-cols-4' })[this.block().columns] ?? '',
  );
}

/** Editor preview (F023): whether a text is shown in French because its English one is empty. */
function isFallback(block: PublicBlock, name: string): boolean {
  return block.fallback?.includes(name) ?? false;
}
