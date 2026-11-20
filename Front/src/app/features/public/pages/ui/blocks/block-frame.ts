import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';

import { Background, Spacing } from '../../data-access/pages.models';

const BACKGROUNDS: Record<Background, string> = {
  background: 'bg-background text-foreground',
  muted: 'bg-muted text-foreground',
  primary: 'bg-primary text-primary-foreground',
};

const SPACINGS: Record<Spacing, string> = {
  small: 'py-6',
  medium: 'py-12',
  large: 'py-20',
};

/** Common frame of every block: theme background role, spacing, hidden on mobile. */
@Component({
  selector: 'app-block-frame',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '[class]': 'classes()' },
  template: `<ng-content />`,
})
export class BlockFrame {
  readonly background = input<Background>('background');
  readonly spacing = input<Spacing>('medium');
  readonly hideOnMobile = input(false);

  protected readonly classes = computed(
    () =>
      `block ${BACKGROUNDS[this.background()]} ${SPACINGS[this.spacing()]}` +
      (this.hideOnMobile() ? ' max-md:hidden' : ''),
  );
}
