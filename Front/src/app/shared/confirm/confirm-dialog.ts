import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { HlmAlertDialogImports } from '@spartan-ng/helm/alert-dialog';

/**
 * Confirmation before an action that cannot be undone simply (delete, restore…). Keyboard
 * accessible; the focus starts on "Annuler" so that Enter never confirms by accident.
 */
@Component({
  selector: 'app-confirm-dialog',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [HlmAlertDialogImports],
  template: `
    <hlm-alert-dialog
      [state]="open() ? 'open' : 'closed'"
      autoFocus="[data-autofocus]"
      (closed)="cancelled.emit()"
    >
      <hlm-alert-dialog-content *hlmAlertDialogPortal="let ctx">
        <hlm-alert-dialog-header>
          <h2 hlmAlertDialogTitle>{{ title() }}</h2>
          <p hlmAlertDialogDescription>{{ description() }}</p>
        </hlm-alert-dialog-header>
        <hlm-alert-dialog-footer>
          <button hlmAlertDialogCancel data-autofocus>Annuler</button>
          <button
            hlmAlertDialogAction
            [variant]="destructive() ? 'destructive' : 'default'"
            (click)="confirmed.emit()"
          >
            {{ confirmLabel() }}
          </button>
        </hlm-alert-dialog-footer>
      </hlm-alert-dialog-content>
    </hlm-alert-dialog>
  `,
})
export class ConfirmDialog {
  readonly open = input(false);
  readonly title = input.required<string>();
  readonly description = input('');
  readonly confirmLabel = input('Confirmer');
  readonly destructive = input(false);

  readonly confirmed = output<void>();
  readonly cancelled = output<void>();
}
