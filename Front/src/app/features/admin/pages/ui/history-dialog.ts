import { DatePipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { HlmDialogImports } from '@spartan-ng/helm/dialog';

import { PageVersion } from '../data-access/admin-pages.models';

/** Published versions of the page, most recent first, each one can be restored (F023). */
@Component({
  selector: 'app-history-dialog',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [DatePipe, HlmButtonImports, HlmDialogImports],
  template: `
    <hlm-dialog [state]="open() ? 'open' : 'closed'" (closed)="closed.emit()">
      <hlm-dialog-content *hlmDialogPortal="let ctx" class="max-h-[90vh] overflow-y-auto">
        <hlm-dialog-header>
          <h2 hlmDialogTitle>Historique des publications</h2>
          <p hlmDialogDescription>
            Restaurer une version la remet dans le brouillon ; le site ne change qu’à la
            publication.
          </p>
        </hlm-dialog-header>
        @if (versions().length === 0) {
          <p class="text-sm text-muted-foreground">Aucune publication pour l’instant.</p>
        }
        <ul class="flex flex-col divide-y" aria-label="Versions publiées">
          @for (version of versions(); track version.number) {
            <li class="flex items-center justify-between gap-3 py-2">
              <span>
                Version {{ version.number }}
                <span class="text-sm text-muted-foreground">
                  · {{ version.published_at | date: 'dd/MM/yyyy à HH:mm' }}
                </span>
              </span>
              <button
                hlmBtn
                variant="outline"
                size="sm"
                type="button"
                [attr.aria-label]="'Restaurer la version ' + version.number"
                (click)="restore.emit(version.number)"
              >
                Restaurer
              </button>
            </li>
          }
        </ul>
      </hlm-dialog-content>
    </hlm-dialog>
  `,
})
export class HistoryDialog {
  readonly open = input(false);
  readonly versions = input<PageVersion[]>([]);
  readonly restore = output<number>();
  readonly closed = output<void>();
}
