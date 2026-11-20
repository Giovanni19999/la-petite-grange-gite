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
import { NgTemplateOutlet, isPlatformBrowser } from '@angular/common';

import { PublicImage } from '../../features/public/pages/data-access/pages.models';
import { ResponsiveImage } from './responsive-image';

/** Horizontal distance, in pixels, for a swipe to change the photo. */
const SWIPE_THRESHOLD_PX = 40;

/**
 * Photos that scroll (F024 v2): dots, swipe and arrow keys, no on-screen arrows or pause
 * button; optional automatic scrolling, stopped on hover, on focus, while dragging and when
 * the device asks for reduced motion, and restarted from zero after each manual move. Only the photo shown (and the next one, just before it is shown) is downloaded.
 *
 * Simulated loop: the track holds a copy of the last photo before the first one and a copy of
 * the first photo after the last one, so that going past an end slides on in the same
 * direction (and a drag shows the neighbour photo); once there, the track jumps without
 * animation to the real photo.
 */
@Component({
  selector: 'app-carousel',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [NgTemplateOutlet, ResponsiveImage],
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
        [class.transition-transform]="animate() && drag() === null"
        [class.duration-500]="animate() && drag() === null"
        [style.transform]="'translateX(calc(-' + position() * 100 + '% + ' + (drag() ?? 0) + 'px))'"
        [attr.aria-live]="playing() ? 'off' : 'polite'"
        (transitionend)="onTransitionEnd()"
      >
        <!-- Simulated loop: a copy of the last photo before the first one... -->
        @if (count() > 1) {
          <ng-container
            *ngTemplateOutlet="
              copy;
              context: { $implicit: count() - 1, shown: index() === 0 || position() < 1 }
            "
          />
        }
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
        <!-- ...and a copy of the first photo after the last one. -->
        @if (count() > 1) {
          <ng-container
            *ngTemplateOutlet="
              copy;
              context: { $implicit: 0, shown: index() === count() - 1 || position() > count() }
            "
          />
        }
      </div>

      <ng-template #copy let-i let-shown="shown">
        <figure
          class="relative h-full w-full shrink-0"
          aria-hidden="true"
          inert
          data-testid="slide-copy"
        >
          @if (shown && loaded().has(i)) {
            <app-responsive-image
              [image]="images()[i]"
              [sizes]="sizes()"
              imageClass="h-full w-full object-cover"
            />
          }
          @if (showCaptions() && images()[i].caption) {
            <figcaption
              class="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent px-4 pt-8 pb-10 text-sm text-white"
            >
              {{ images()[i].caption }}
            </figcaption>
          }
        </figure>
      </ng-template>

      @if (images().length > 1) {
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

  protected readonly count = computed(() => this.images().length);
  /** Position on the track: 0 is the copy of the last photo, 1 the first photo… */
  protected readonly position = signal(1);
  /** Photo shown (0-based), copies counted as the photo they copy. */
  protected readonly index = computed(() => mod(this.position() - 1, Math.max(this.count(), 1)));
  /** Slide animation on; off for the jump from a copy to the real photo. */
  protected readonly animate = signal(true);
  /** Photos already requested (the current one, the ones seen, the next one about to show). */
  protected readonly loaded = signal(new Set([0]));
  protected readonly hovered = signal(false);
  protected readonly focused = signal(false);
  protected readonly reducedMotion = signal(false);
  /** Horizontal finger (or mouse) movement while dragging, in pixels; `null` when idle. */
  protected readonly drag = signal<number | null>(null);
  private dragStart: { x: number; y: number; pointer: number } | null = null;

  /** Scrolling by itself right now. */
  protected readonly playing = computed(
    () =>
      this.autoplay() &&
      this.images().length > 1 &&
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
      // Reading the index restarts the delay after every change of photo.
      this.markLoaded(this.index() + 1);
      const timer = setInterval(() => this.step(1), this.interval() * 1000);
      onCleanup(() => clearInterval(timer));
    });
  }

  /** Shows photo `target` (0-based), sliding the shortest way inside the track. */
  go(target: number): void {
    const count = this.count();
    if (count === 0) {
      return;
    }
    const photo = mod(target, count);
    this.markLoaded(photo);
    this.position.set(photo + 1);
  }

  /** Next (`1`) or previous (`-1`) photo; past an end, slides on to the copy. */
  step(delta: 1 | -1): void {
    const count = this.count();
    if (count < 2) {
      return;
    }
    this.settle();
    this.markLoaded(this.index() + delta);
    if (this.reducedMotion()) {
      // No animation: no need to pass through the copies.
      this.position.set(mod(this.index() + delta, count) + 1);
      return;
    }
    this.position.update((position) => position + delta);
  }

  /** After a slide onto a copy, jump without animation to the photo it copies. */
  protected onTransitionEnd(): void {
    this.settle();
  }

  private settle(): void {
    const count = this.count();
    const position = this.position();
    if (position >= 1 && position <= count) {
      return;
    }
    this.animate.set(false);
    this.position.set(this.index() + 1);
    // Turn the animation back on once the jump has been painted.
    const restore = () => this.animate.set(true);
    if (typeof requestAnimationFrame === 'function') {
      requestAnimationFrame(() => requestAnimationFrame(restore));
    } else {
      setTimeout(restore);
    }
  }

  protected onKey(event: KeyboardEvent): void {
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault();
      this.step(event.key === 'ArrowLeft' ? -1 : 1);
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
      this.step(distance < 0 ? 1 : -1);
    }
  }

  /** The browser took the gesture over (scrolling) or the pointer left: back to rest. */
  protected cancelDrag(): void {
    this.dragStart = null;
    this.drag.set(null);
  }

  private markLoaded(target: number): void {
    const index = mod(target, this.count());
    if (!this.loaded().has(index)) {
      this.loaded.update((loaded) => new Set([...loaded, index]));
    }
  }
}

/** Remainder always between 0 and `count - 1`. */
function mod(value: number, count: number): number {
  return ((value % count) + count) % count;
}
