import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';

import { Carousel } from '../../../../../shared/media/carousel';
import { ResponsiveImage } from '../../../../../shared/media/responsive-image';
import {
  Align,
  CarouselBlock,
  ColumnsBlock,
  GalleryBlock,
  HeroBlock,
  ImageBlock,
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

/** Large banner: photo (darkened), title (the page h1), subtitle and call to action. */
@Component({
  selector: 'app-hero-block',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [Carousel, ResponsiveImage],
  template: `
    <div
      class="relative isolate flex flex-col justify-center gap-4 overflow-hidden bg-gradient-to-br from-primary to-primary/70 px-4 py-16 text-primary-foreground"
      [class]="heightClass() + ' ' + alignClass()"
    >
      @if (block().mode === 'carousel' && block().images.length > 0) {
        <div class="absolute inset-0 -z-10">
          <app-carousel
            class="h-full"
            label="Photos du gîte"
            [images]="block().images"
            [autoplay]="block().autoplay"
            [interval]="block().interval"
            [priority]="priority()"
            [showCaptions]="false"
          />
          <div
            class="pointer-events-none absolute inset-0 bg-black"
            [style.opacity]="block().darken / 100"
          ></div>
        </div>
      } @else if (block().image; as image) {
        <div class="absolute inset-0 -z-10">
          <app-responsive-image
            [image]="image"
            sizes="100vw"
            [priority]="priority()"
            imageClass="h-full w-full object-cover"
          />
          <div class="absolute inset-0 bg-black" [style.opacity]="block().darken / 100"></div>
        </div>
      }
      <!-- Clicks and swipes go through the texts to the photos (F024); the button stays usable. -->
      <div
        class="pointer-events-none mx-auto flex w-full max-w-4xl flex-col gap-4"
        [class]="alignClass()"
      >
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
            class="pointer-events-auto mt-2 inline-flex w-fit rounded-md bg-background px-6 py-3 font-medium text-foreground shadow-sm transition hover:opacity-90"
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
  /** First image of the page (loaded at once). */
  readonly priority = input(false);
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
  imports: [ResponsiveImage],
  template: `
    <div class="mx-auto grid max-w-5xl gap-8 px-4" [class]="gridClass()">
      <div
        class="rich-text"
        [class.opacity-60]="isFallback(block(), 'body')"
        [innerHTML]="block().body"
      ></div>
      @if (block().image; as image) {
        <figure
          class="flex flex-col gap-2"
          [class]="block().image_position === 'left' ? '@3xl:order-first' : ''"
        >
          <div class="aspect-[4/3] overflow-hidden rounded-lg bg-muted">
            <app-responsive-image
              [image]="image"
              sizes="(min-width: 768px) 50vw, 100vw"
              [priority]="priority()"
              imageClass="h-full w-full object-cover"
            />
          </div>
          @if (image.caption) {
            <figcaption class="text-sm text-muted-foreground">{{ image.caption }}</figcaption>
          }
        </figure>
      }
    </div>
  `,
})
export class TextImageBlockView {
  readonly block = input.required<TextImageBlock>();
  /** First image of the page (loaded at once). */
  readonly priority = input(false);
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

/** Image grid: square thumbnails cut around each photo's focal point, with captions. */
@Component({
  selector: 'app-gallery-block',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ResponsiveImage],
  template: `
    @if (block().images.length > 0) {
      <div class="mx-auto grid max-w-5xl grid-cols-2 gap-2 px-4" [class]="columnsClass()">
        @for (image of block().images; track image.id; let first = $first) {
          <figure class="flex flex-col gap-1">
            <div class="aspect-square overflow-hidden rounded-md bg-muted">
              <app-responsive-image
                [image]="image"
                [sizes]="sizes()"
                [priority]="priority() && first"
                imageClass="h-full w-full object-cover"
              />
            </div>
            @if (image.caption) {
              <figcaption class="text-sm text-muted-foreground">{{ image.caption }}</figcaption>
            }
          </figure>
        }
      </div>
    }
  `,
})
export class GalleryBlockView {
  readonly block = input.required<GalleryBlock>();
  /** First image of the page (loaded at once). */
  readonly priority = input(false);
  /** One column out of 2 on phones, out of the chosen number from 768 px. */
  protected readonly sizes = computed(
    () => `(min-width: 768px) ${Math.round(100 / this.block().columns)}vw, 50vw`,
  );
  protected readonly columnsClass = computed(
    () => ({ 3: '@3xl:grid-cols-3', 4: '@3xl:grid-cols-4' })[this.block().columns] ?? '',
  );
}

const IMAGE_WIDTH: Record<ImageBlock['width'], { frame: string; sizes: string }> = {
  narrow: { frame: 'mx-auto max-w-2xl px-4', sizes: '(min-width: 672px) 672px, 100vw' },
  wide: { frame: 'mx-auto max-w-5xl px-4', sizes: '(min-width: 1024px) 1024px, 100vw' },
  full: { frame: 'w-full', sizes: '100vw' },
};

/** One photo with its caption, narrow, wide or full width (F024). */
@Component({
  selector: 'app-image-block',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ResponsiveImage],
  template: `
    @if (block().image; as image) {
      <figure class="flex flex-col gap-2" [class]="width().frame">
        <app-responsive-image
          [image]="image"
          [sizes]="width().sizes"
          [priority]="priority()"
          [imageClass]="block().width === 'full' ? 'h-auto w-full' : 'h-auto w-full rounded-lg'"
        />
        @if (image.caption) {
          <figcaption class="text-sm text-muted-foreground" [class.px-4]="block().width === 'full'">
            {{ image.caption }}
          </figcaption>
        }
      </figure>
    }
  `,
})
export class ImageBlockView {
  readonly block = input.required<ImageBlock>();
  /** First image of the page (loaded at once). */
  readonly priority = input(false);
  protected readonly width = computed(() => IMAGE_WIDTH[this.block().width]);
}

/** Photos that scroll, with their captions (F024). */
@Component({
  selector: 'app-carousel-block',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [Carousel],
  template: `
    @if (block().images.length > 0) {
      <div class="mx-auto max-w-5xl px-4">
        <app-carousel
          class="overflow-hidden rounded-lg bg-muted"
          [class]="heightClass()"
          label="Photos"
          [images]="block().images"
          [autoplay]="block().autoplay"
          [interval]="block().interval"
          [priority]="priority()"
          sizes="(min-width: 1024px) 1024px, 100vw"
        />
      </div>
    }
  `,
})
export class CarouselBlockView {
  readonly block = input.required<CarouselBlock>();
  /** First image of the page (loaded at once). */
  readonly priority = input(false);
  protected readonly heightClass = computed(() => HERO_HEIGHT[this.block().height]);
}

/** Editor preview (F023): whether a text is shown in French because its English one is empty. */
function isFallback(block: PublicBlock, name: string): boolean {
  return block.fallback?.includes(name) ?? false;
}
