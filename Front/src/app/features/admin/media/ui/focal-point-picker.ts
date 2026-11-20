import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';

import { thumbnailUrl } from '../data-access/media.api';
import { FocalPoint, Media } from '../data-access/media.models';

/** Step of the arrow keys, in percent. */
const KEY_STEP = 5;

/**
 * Click (or use the arrow keys) on the important part of the photo: it stays visible in every
 * frame shape (F003). Coordinates are percentages of the width and height.
 */
@Component({
  selector: 'app-focal-point-picker',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="flex flex-col gap-2">
      <span class="text-sm font-medium" [id]="labelId">Point important de la photo</span>
      <div
        class="relative cursor-crosshair overflow-hidden rounded-md bg-muted outline-offset-2 focus-visible:outline-2 focus-visible:outline-ring"
        role="slider"
        tabindex="0"
        [attr.aria-labelledby]="labelId"
        [attr.aria-valuetext]="'horizontal ' + value().x + ' %, vertical ' + value().y + ' %'"
        [attr.aria-valuenow]="value().x"
        aria-valuemin="0"
        aria-valuemax="100"
        data-testid="focal-area"
        (click)="pick($event)"
        (keydown)="onKey($event)"
      >
        <img class="block h-auto w-full select-none" [src]="url()" alt="" draggable="false" />
        <span
          class="pointer-events-none absolute size-6 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white bg-primary/70 shadow"
          [style.left.%]="value().x"
          [style.top.%]="value().y"
        ></span>
      </div>
      <p class="text-xs text-muted-foreground">
        Cliquez sur le sujet (ou utilisez les flèches du clavier) : il restera visible quel que soit
        le cadre.
      </p>
    </div>
  `,
})
export class FocalPointPicker {
  readonly media = input.required<Media>();
  readonly value = input.required<FocalPoint>();
  readonly changed = output<FocalPoint>();

  protected readonly labelId = `focal-${Math.random().toString(36).slice(2)}`;
  protected readonly url = computed(() => thumbnailUrl(this.media(), 960));

  protected pick(event: MouseEvent): void {
    const area = (event.currentTarget as HTMLElement).getBoundingClientRect();
    if (area.width === 0 || area.height === 0) {
      return;
    }
    this.changed.emit({
      x: clamp(((event.clientX - area.left) / area.width) * 100),
      y: clamp(((event.clientY - area.top) / area.height) * 100),
    });
  }

  protected onKey(event: KeyboardEvent): void {
    const { x, y } = this.value();
    const moves: Record<string, FocalPoint> = {
      ArrowLeft: { x: x - KEY_STEP, y },
      ArrowRight: { x: x + KEY_STEP, y },
      ArrowUp: { x, y: y - KEY_STEP },
      ArrowDown: { x, y: y + KEY_STEP },
    };
    const next = moves[event.key];
    if (next) {
      event.preventDefault();
      this.changed.emit({ x: clamp(next.x), y: clamp(next.y) });
    }
  }
}

function clamp(value: number): number {
  return Math.round(Math.min(100, Math.max(0, value)));
}
