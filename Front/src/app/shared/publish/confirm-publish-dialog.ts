import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { HlmAlertDialogImports } from '@spartan-ng/helm/alert-dialog';

import { PublishSummary } from './publish-summary';

const LANGUAGE_NAMES = { fr: 'français', en: 'anglais' } as const;

/**
 * Mandatory confirmation before anything changes on the public site (ADR-0009): shows the
 * summary of changes and asks to publish. Shared by every feature that publishes; the focus
 * starts on "Annuler".
 */
@Component({
  selector: 'app-confirm-publish-dialog',
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
          <h2 hlmAlertDialogTitle>Publier sur le site ?</h2>
          <div hlmAlertDialogDescription class="flex flex-col gap-3 text-left">
            @if (notice(); as message) {
              <p class="rounded-md bg-muted p-2 text-foreground" role="alert">{{ message }}</p>
            }
            <p>Les visiteurs verront ces changements dès la publication :</p>
            <ul class="flex flex-col gap-1 text-foreground" data-testid="publish-summary">
              @for (line of summary()?.lines ?? []; track line.label) {
                <li>
                  <span class="font-medium">{{ line.label }}</span>
                  @if (line.items.length > 0) {
                    : {{ line.items.join(', ') }}
                  }
                </li>
              }
            </ul>
            @if (languages()) {
              <p>Langues concernées : {{ languages() }}.</p>
            }
          </div>
        </hlm-alert-dialog-header>
        <hlm-alert-dialog-footer>
          <button hlmAlertDialogCancel data-autofocus>Annuler</button>
          <button hlmAlertDialogAction [disabled]="publishing()" (click)="confirmed.emit()">
            {{ publishing() ? 'Publication…' : 'Publier' }}
          </button>
        </hlm-alert-dialog-footer>
      </hlm-alert-dialog-content>
    </hlm-alert-dialog>
  `,
})
export class ConfirmPublishDialog {
  readonly open = input(false);
  readonly summary = input<PublishSummary | null>(null);
  /** Shown above the summary, e.g. when the content changed since the dialog opened. */
  readonly notice = input<string | null>(null);
  readonly publishing = input(false);

  readonly confirmed = output<void>();
  readonly cancelled = output<void>();

  protected readonly languages = computed(() =>
    (this.summary()?.languages ?? []).map((lang) => LANGUAGE_NAMES[lang]).join(' et '),
  );
}
