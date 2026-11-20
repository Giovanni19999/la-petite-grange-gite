import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  PLATFORM_ID,
  computed,
  effect,
  inject,
  input,
  signal,
} from '@angular/core';
import { isPlatformBrowser } from '@angular/common';

import { PublicImage } from '../../features/public/pages/data-access/pages.models';
import { ResponsiveImage } from './responsive-image';

/** Horizontal distance, in pixels, for a swipe to change the photo. */
const SWIPE_THRESHOLD_PX = 40;

/**
 * Photos that scroll (F024): arrows, dots, swipe and arrow keys; optional automatic scrolling
 * with a pause button, stopped on hover, on focus and when the device asks for reduced
 * motion. Only the photo shown (and the next one, just before it is shown) is downloaded.
 */
@Component({
  selector: 'app-carousel',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [ResponsiveImage],
  host: { class: 'block' },
  template: `
    <div
      class="group/carousel relative h-full w-full touch-pan-y overflow-hidden outline-offset-2 select-none focus-visible:outline-2 focus-visible:outline-ring"
      role="region"
      aria-roledescription="carrousel"
      [attr.aria-label]="label()"
      tabindex="0"
      (keydown)="onKey($event)"
      (mouseenter)="hovered.set(true)"
      (mouseleave)="hovered.set(false)"
      (focusin)="focused.set(true)"
      (focusout)="focused.set(false)"
      (pointerdown)="onPointerDown($event)"
      (pointermove)="onPointerMove($event)"
      (pointerup)="onPointerUp($event)"
      (pointercancel)="cancelDrag()"
      (dragstart)="$event.preventDefault()"
    >
      <div
        class="flex h-full ease-out motion-reduce:transition-none"
        [class.transition-transform]="drag() === null"
        [class.duration-500]="drag() === null"
        [style.transform]="'translateX(calc(-' + index() * 100 + '% + ' + (drag() ?? 0) + 'px))'"
        [attr.aria-live]="playing() ? 'off' : 'polite'"
      >
        @for (image of images(); track image.id; let i = $index) {
          <figure
            class="relative h-full w-full shrink-0"
            role="group"
            aria-roledescription="diapositive"
            [attr.aria-label]="i + 1 + ' sur ' + images().length"
            [attr.aria-hidden]="i !== index()"
            [attr.inert]="i !== index() ? '' : null"
            data-testid="slide"
          >
            @if (loaded().has(i)) {
              <app-responsive-image
                [image]="image"
                [sizes]="sizes()"
                [priority]="priority() && i === 0"
                imageClass="h-full w-full object-cover"
              />
            }
            @if (showCaptions() && image.caption) {
              <figcaption
                class="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent px-4 pt-8 pb-10 text-sm text-white"
              >
                {{ image.caption }}
              </figcaption>
            }
          </figure>
        }
      </div>

      @if (images().length > 1) {
        <button
          type="button"
          class="absolute top-1/2 left-2 z-10 flex size-10 -translate-y-1/2 items-center justify-center rounded-full bg-background/80 text-foreground shadow hover:bg-background"
          aria-label="Photo précédente"
          (click)="go(index() - 1)"
        >
          ‹
        </button>
        <button
          type="button"
          class="absolute top-1/2 right-2 z-10 flex size-10 -translate-y-1/2 items-center justify-center rounded-full bg-background/80 text-foreground shadow hover:bg-background"
          aria-label="Photo suivante"
          (click)="go(index() + 1)"
        >
          ›
        </button>
        <div class="absolute inset-x-0 bottom-2 z-10 flex items-center justify-center gap-2">
          @for (image of images(); track image.id; let i = $index) {
            <button
              type="button"
              class="size-3 rounded-full border border-white/80 shadow"
              [class.bg-white]="i === index()"
              [class.bg-white/30]="i !== index()"
              [attr.aria-label]="'Photo ' + (i + 1) + ' sur ' + images().length"
              [attr.aria-current]="i === index() ? 'true' : null"
              data-testid="dot"
              (click)="go(i)"
            ></button>
          }
          @if (autoplay() && !reducedMotion()) {
            <button
              type="button"
              class="ml-2 rounded-full bg-background/80 px-3 py-1 text-xs text-foreground shadow"
              [attr.aria-label]="
                paused() ? 'Reprendre le défilement' : 'Mettre en pause le défilement'
              "
              data-testid="pause"
              (click)="paused.set(!paused())"
            >
              {{ paused() ? '▶ Lecture' : '❚❚ Pause' }}
            </button>
          }
        </div>
      }
    </div>
  `,
})
export class Carousel {
  readonly images = input.required<PublicImage[]>();
  /** Accessible name of the carousel. */
  readonly label = input('Photos');
  readonly autoplay = input(false);
  /** Seconds between two photos. */
  readonly interval = input(6);
  readonly sizes = input('100vw');
  /** The first photo is the first image of the page (loaded at once). */
  readonly priority = input(false);
  readonly showCaptions = input(true);

  protected readonly index = signal(0);
  /** Photos already requested (the current one, the ones seen, the next one about to show). */
  protected readonly loaded = signal(new Set([0]));
  protected readonly hovered = signal(false);
  protected readonly focused = signal(false);
  protected readonly paused = signal(false);
  protected readonly reducedMotion = signal(false);
  /** Horizontal finger (or mouse) movement while dragging, in pixels; `null` when idle. */
  protected readonly drag = signal<number | null>(null);
  private dragStart: { x: number; y: number; pointer: number } | null = null;

  /** Scrolling by itself right now. */
  protected readonly playing = computed(
    () =>
      this.autoplay() &&
      this.images().length > 1 &&
      !this.paused() &&
      !this.hovered() &&
      !this.focused() &&
      this.drag() === null &&
      !this.reducedMotion(),
  );

  constructor() {
    const browser = isPlatformBrowser(inject(PLATFORM_ID));
    if (browser && typeof matchMedia === 'function') {
      const query = matchMedia('(prefers-reduced-motion: reduce)');
      this.reducedMotion.set(query.matches);
      const listener = (event: MediaQueryListEvent) => this.reducedMotion.set(event.matches);
      query.addEventListener('change', listener);
      inject(DestroyRef).onDestroy(() => query.removeEventListener('change', listener));
    }
    // Automatic scrolling, in the browser only: the next photo is requested while the
    // current one is shown, then shown after the delay.
    effect((onCleanup) => {
      if (!browser || !this.playing()) {
        return;
      }
      this.markLoaded(this.index() + 1);
      const timer = setInterval(() => this.go(this.index() + 1), this.interval() * 1000);
      onCleanup(() => clearInterval(timer));
    });
  }

  /** Shows photo `target` (wrapping around at both ends). */
  go(target: number): void {
    const count = this.images().length;
    if (count === 0) {
      return;
    }
    const next = ((target % count) + count) % count;
    this.markLoaded(next);
    this.index.set(next);
  }

  protected onKey(event: KeyboardEvent): void {
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault();
      this.go(this.index() + (event.key === 'ArrowLeft' ? -1 : 1));
    }
  }

  // Swipe (F024): the browser keeps vertical scrolling (`touch-action: pan-y`), horizontal
  // moves drag the photos, which follow the finger; a long enough move changes the photo.

  protected onPointerDown(event: PointerEvent): void {
    if (event.button !== 0 || (event.target as Element).closest('button')) {
      return;
    }
    this.dragStart = { x: event.clientX, y: event.clientY, pointer: event.pointerId };
    this.markLoaded(this.index() + 1);
    this.markLoaded(this.index() - 1);
  }

  protected onPointerMove(event: PointerEvent): void {
    const start = this.dragStart;
    if (!start || event.pointerId !== start.pointer) {
      return;
    }
    const dx = event.clientX - start.x;
    if (this.drag() === null && Math.abs(dx) < Math.abs(event.clientY - start.y)) {
      return;
    }
    this.drag.set(dx);
  }

  protected onPointerUp(event: PointerEvent): void {
    const start = this.dragStart;
    if (!start || event.pointerId !== start.pointer) {
      return;
    }
    const distance = event.clientX - start.x;
    this.cancelDrag();
    if (Math.abs(distance) >= SWIPE_THRESHOLD_PX) {
      this.go(this.index() + (distance < 0 ? 1 : -1));
    }
  }

  /** The browser took the gesture over (scrolling) or the pointer left: back to rest. */
  protected cancelDrag(): void {
    this.dragStart = null;
    this.drag.set(null);
  }

  private markLoaded(target: number): void {
    const count = this.images().length;
    const index = ((target % count) + count) % count;
    if (!this.loaded().has(index)) {
      this.loaded.update((loaded) => new Set([...loaded, index]));
    }
  }
}
