import { CdkDrag, CdkDragDrop, CdkDragHandle, CdkDropList } from '@angular/cdk/drag-drop';
import { NgTemplateOutlet } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  effect,
  inject,
  input,
  numberAttribute,
  signal,
} from '@angular/core';
import { RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { HlmSheetImports } from '@spartan-ng/helm/sheet';

import { ConfirmDialog } from '../../../../shared/confirm/confirm-dialog';
import { ConfirmPublishDialog } from '../../../../shared/publish/confirm-publish-dialog';
import { PublishSummary } from '../../../../shared/publish/publish-summary';
import { Lang } from '../../../public/pages/data-access/pages.models';
import { BlockRenderer } from '../../../public/pages/ui/blocks/block-renderer';
import { BlockType, PageVersion } from '../data-access/admin-pages.models';
import {
  BLOCK_TYPES,
  blockLabel,
  publishSummary,
  toPreviewBlock,
} from '../data-access/page-content';
import { MediaStore } from '../../media/data-access/media.store';
import { PageEditorStore, SaveStatus } from '../data-access/page-editor.store';
import { BlockPanel } from '../ui/block-panel';
import { HistoryDialog } from '../ui/history-dialog';
import { PageSettingsDialog } from '../ui/page-settings-dialog';

export type Device = 'desktop' | 'tablet' | 'phone';

/** Width of the preview per device; desktop takes the available width. */
const DEVICE_WIDTHS: Record<Device, string> = {
  desktop: '100%',
  tablet: '768px',
  phone: '375px',
};

const DEVICES: readonly { value: Device; label: string }[] = [
  { value: 'desktop', label: 'Ordinateur' },
  { value: 'tablet', label: 'Tablette' },
  { value: 'phone', label: 'Téléphone' },
];

const STATUS_LABELS: Record<SaveStatus, string> = {
  saved: 'Enregistré',
  pending: 'Modifications en cours…',
  saving: 'Enregistrement…',
  invalid: 'Non enregistré : à corriger',
  conflict: 'Page modifiée ailleurs',
  error: 'Échec de l’enregistrement',
};

/** Screens at least this wide show the block panel beside the preview; others use a drawer. */
const WIDE_SCREEN_QUERY = '(min-width: 1024px)';

/**
 * Visual page editor (F023): live preview of the draft drawn with the public block
 * components, panel for the selected block, autosave, publication with confirmation.
 */
@Component({
  selector: 'app-page-editor-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  providers: [PageEditorStore],
  imports: [
    CdkDropList,
    CdkDrag,
    CdkDragHandle,
    NgTemplateOutlet,
    RouterLink,
    HlmButtonImports,
    HlmSheetImports,
    BlockRenderer,
    BlockPanel,
    ConfirmDialog,
    ConfirmPublishDialog,
    HistoryDialog,
    PageSettingsDialog,
  ],
  host: { '(window:beforeunload)': 'onBeforeUnload($event)' },
  template: `
    @if (store.loadError()) {
      <section class="mx-auto max-w-3xl px-4 py-10">
        <p class="text-destructive" role="alert">Impossible d’ouvrir cette page.</p>
        <a routerLink="/admin/pages" class="text-sm hover:underline">← Pages du site</a>
      </section>
    } @else if (store.content(); as content) {
      <div class="flex h-dvh flex-col">
        <!-- Toolbar -->
        <header class="flex flex-wrap items-center gap-2 border-b bg-background px-3 py-2">
          <a
            routerLink="/admin/pages"
            class="text-sm text-muted-foreground hover:underline"
            aria-label="Retour aux pages du site"
            >← Pages</a
          >
          <h1 class="mr-auto truncate font-semibold">{{ content.settings.title.fr }}</h1>

          <div class="flex rounded-md bg-muted p-1" role="group" aria-label="Langue des textes">
            @for (lang of langs; track lang) {
              <button
                type="button"
                class="rounded px-2 py-1 text-sm aria-pressed:bg-background aria-pressed:font-medium aria-pressed:shadow-sm"
                [attr.aria-pressed]="store.lang() === lang"
                (click)="store.lang.set(lang)"
              >
                {{ lang.toUpperCase() }}
              </button>
            }
          </div>
          <div class="flex rounded-md bg-muted p-1" role="group" aria-label="Aperçu">
            @for (option of devices; track option.value) {
              <button
                type="button"
                class="rounded px-2 py-1 text-sm aria-pressed:bg-background aria-pressed:font-medium aria-pressed:shadow-sm"
                [attr.aria-pressed]="device() === option.value"
                (click)="device.set(option.value)"
              >
                {{ option.label }}
              </button>
            }
          </div>

          <span class="text-sm" role="status" data-testid="save-status" [class]="statusClass()">
            {{ statusLabel() }}
          </span>
          @if (store.status() === 'error') {
            <button hlmBtn variant="outline" size="sm" type="button" (click)="store.retry()">
              Réessayer
            </button>
          }
          <button hlmBtn variant="outline" size="sm" type="button" (click)="settingsOpen.set(true)">
            Réglages
          </button>
          <button hlmBtn variant="outline" size="sm" type="button" (click)="openHistory()">
            Historique
          </button>
          <button hlmBtn size="sm" type="button" [disabled]="!canPublish()" (click)="openPublish()">
            Publier
          </button>
        </header>

        <!-- Messages -->
        @if (store.status() === 'conflict') {
          <div class="flex flex-wrap items-center gap-3 bg-amber-50 px-4 py-2 text-sm" role="alert">
            Cette page a été modifiée ailleurs (autre onglet ou autre propriétaire). Vos dernières
            modifications ne sont pas enregistrées.
            <button hlmBtn size="sm" type="button" (click)="store.reloadAfterConflict()">
              Charger la dernière version
            </button>
          </div>
        }
        @if (store.lostChanges().length > 0) {
          <div
            class="flex flex-wrap items-center gap-3 bg-amber-50 px-4 py-2 text-sm"
            role="status"
          >
            Dernière version chargée. Modifications à refaire :
            {{ store.lostChanges().join(', ') }}.
            <button type="button" class="underline" (click)="store.lostChanges.set([])">OK</button>
          </div>
        }
        @if (store.problems().other.length > 0 || store.problems().slug) {
          <div class="bg-destructive/10 px-4 py-2 text-sm text-destructive" role="alert">
            @for (problem of store.problems().other; track problem) {
              <p>{{ problem }}</p>
            }
            @if (store.problems().slug) {
              <p>
                Adresse de la page à corriger.
                <button type="button" class="underline" (click)="settingsOpen.set(true)">
                  Ouvrir les réglages
                </button>
              </p>
            }
          </div>
        }
        @if (message(); as text) {
          <div class="bg-primary/10 px-4 py-2 text-sm" role="status">{{ text }}</div>
        }

        <div class="flex min-h-0 flex-1">
          <!-- Preview -->
          <main
            class="min-w-0 flex-1 overflow-y-auto bg-muted/50 p-2 sm:p-4"
            aria-label="Aperçu de la page"
          >
            <div
              class="@container mx-auto bg-background shadow-sm transition-[max-width]"
              data-testid="preview"
              [style.max-width]="previewWidth()"
              [attr.lang]="store.lang()"
            >
              <ng-container *ngTemplateOutlet="addButton; context: { $implicit: 0 }" />
              <div cdkDropList (cdkDropListDropped)="drop($event)">
                @for (block of content.blocks; track block.id; let i = $index; let last = $last) {
                  <div
                    cdkDrag
                    class="group relative"
                    data-testid="preview-block"
                    [attr.data-block-id]="block.id"
                  >
                    <div
                      class="relative cursor-pointer outline-offset-[-2px] hover:outline-2 hover:outline-primary/40"
                      [class.outline-2]="store.selectedId() === block.id"
                      [class.outline-primary]="store.selectedId() === block.id"
                      [class.opacity-50]="!block.visible"
                      role="button"
                      tabindex="0"
                      [attr.aria-label]="
                        'Modifier le bloc ' + label(block.type) + ' (' + (i + 1) + ')'
                      "
                      [attr.aria-pressed]="store.selectedId() === block.id"
                      (click)="select(block.id); $event.preventDefault()"
                      (keydown.enter)="select(block.id)"
                      (keydown.space)="select(block.id); $event.preventDefault()"
                    >
                      <div class="pointer-events-none" inert>
                        <app-block-renderer [block]="preview(block)" />
                      </div>
                      @if (!block.visible) {
                        <span
                          class="absolute top-2 left-2 rounded bg-foreground px-2 py-0.5 text-xs text-background"
                        >
                          Masqué sur le site
                        </span>
                      }
                      @if (block.type === 'gallery' && block.images.length === 0) {
                        <p class="p-6 text-center text-sm text-muted-foreground">
                          Galerie vide : ajoutez des photos dans le panneau du bloc.
                        </p>
                      }
                    </div>
                    <div
                      class="absolute top-2 right-2 z-10 flex gap-1 rounded-md border bg-background p-1 shadow-sm group-hover:opacity-100 focus-within:opacity-100"
                      [class.opacity-0]="store.selectedId() !== block.id"
                      role="toolbar"
                      [attr.aria-label]="
                        'Actions du bloc ' + label(block.type) + ' (' + (i + 1) + ')'
                      "
                    >
                      <button
                        type="button"
                        cdkDragHandle
                        class="cursor-grab rounded px-2 py-1 text-sm hover:bg-muted"
                        aria-label="Glisser pour déplacer"
                        title="Glisser pour déplacer"
                      >
                        ⠿
                      </button>
                      <button
                        type="button"
                        class="rounded px-2 py-1 text-sm hover:bg-muted disabled:opacity-40"
                        aria-label="Monter"
                        title="Monter"
                        [disabled]="i === 0"
                        (click)="move(i, i - 1)"
                      >
                        ↑
                      </button>
                      <button
                        type="button"
                        class="rounded px-2 py-1 text-sm hover:bg-muted disabled:opacity-40"
                        aria-label="Descendre"
                        title="Descendre"
                        [disabled]="last"
                        (click)="move(i, i + 1)"
                      >
                        ↓
                      </button>
                      <button
                        type="button"
                        class="rounded px-2 py-1 text-sm hover:bg-muted"
                        aria-label="Dupliquer"
                        title="Dupliquer"
                        (click)="store.duplicate(block.id)"
                      >
                        ⧉
                      </button>
                      <button
                        type="button"
                        class="rounded px-2 py-1 text-sm text-destructive hover:bg-muted"
                        aria-label="Supprimer"
                        title="Supprimer"
                        (click)="toDelete.set(block.id)"
                      >
                        ✕
                      </button>
                    </div>
                    <ng-container *ngTemplateOutlet="addButton; context: { $implicit: i + 1 }" />
                  </div>
                } @empty {
                  <p class="p-8 text-center text-muted-foreground">
                    Cette page est vide : ajoutez un premier bloc.
                  </p>
                }
              </div>
            </div>
          </main>

          <!-- Block panel: beside the preview on wide screens, drawer on phones -->
          @if (isWide()) {
            <aside
              class="w-96 shrink-0 overflow-y-auto border-l bg-background p-4"
              aria-label="Bloc sélectionné"
            >
              @if (store.selectedBlock(); as block) {
                <app-block-panel
                  [block]="block"
                  [lang]="store.lang()"
                  (changed)="store.updateBlock($event)"
                />
              } @else {
                <p class="text-sm text-muted-foreground">
                  Cliquez sur un bloc de l’aperçu pour modifier ses textes et ses réglages.
                </p>
              }
            </aside>
          }
        </div>
      </div>

      @if (!isWide()) {
        <hlm-sheet
          side="bottom"
          [state]="drawerOpen() ? 'open' : 'closed'"
          (closed)="drawerOpen.set(false)"
        >
          <hlm-sheet-content
            *hlmSheetPortal="let ctx"
            closeLabel="Fermer"
            class="max-h-[85dvh] overflow-y-auto p-4"
          >
            <hlm-sheet-header>
              <h2 hlmSheetTitle>Modifier le bloc</h2>
            </hlm-sheet-header>
            @if (store.selectedBlock(); as block) {
              <app-block-panel
                [block]="block"
                [lang]="store.lang()"
                (changed)="store.updateBlock($event)"
              />
            }
          </hlm-sheet-content>
        </hlm-sheet>
      }

      <app-page-settings-dialog
        [open]="settingsOpen()"
        [settings]="content.settings"
        [isHome]="store.page()?.is_home ?? false"
        [slugError]="store.problems().slug"
        (changed)="store.updateSettings($event)"
        (closed)="settingsOpen.set(false)"
      />
      <app-history-dialog
        [open]="historyOpen()"
        [versions]="versions()"
        (restore)="toRestore.set($event)"
        (closed)="historyOpen.set(false)"
      />
      <app-confirm-dialog
        [open]="toDelete() !== null"
        title="Supprimer ce bloc ?"
        description="Le bloc sera retiré du brouillon. Le site ne change qu’à la publication."
        confirmLabel="Supprimer"
        [destructive]="true"
        (confirmed)="removeBlock()"
        (cancelled)="toDelete.set(null)"
      />
      <app-confirm-dialog
        [open]="toRestore() !== null"
        [title]="'Restaurer la version ' + toRestore() + ' ?'"
        description="Son contenu remplacera le brouillon actuel. Le site ne change qu’à la publication."
        confirmLabel="Restaurer"
        (confirmed)="restore()"
        (cancelled)="toRestore.set(null)"
      />
      <app-confirm-publish-dialog
        [open]="publishOpen()"
        [summary]="summary()"
        [notice]="publishNotice()"
        [publishing]="publishing()"
        (confirmed)="publish()"
        (cancelled)="publishOpen.set(false)"
      />
    }

    <ng-template #addButton let-index>
      <div class="relative flex justify-center py-1">
        <button
          type="button"
          class="rounded-full border bg-background px-3 py-0.5 text-sm text-muted-foreground shadow-sm hover:text-foreground"
          [attr.aria-expanded]="pickerAt() === index"
          [attr.aria-label]="'Ajouter un bloc à la position ' + (index + 1)"
          (click)="pickerAt.set(pickerAt() === index ? null : index)"
        >
          + Ajouter un bloc
        </button>
      </div>
      @if (pickerAt() === index) {
        <div
          class="mx-2 mb-2 grid gap-2 rounded-lg border bg-background p-2 shadow-sm @md:grid-cols-2"
          role="group"
          aria-label="Type de bloc"
        >
          @for (choice of blockTypes; track choice.type) {
            <button
              type="button"
              class="flex flex-col items-start rounded-md border p-2 text-left hover:border-primary"
              (click)="add(choice.type, index)"
            >
              <span class="font-medium">{{ choice.label }}</span>
              <span class="text-xs text-muted-foreground">{{ choice.description }}</span>
            </button>
          }
        </div>
      }
    </ng-template>
  `,
})
export class PageEditorPage {
  protected readonly store = inject(PageEditorStore);
  private readonly media = inject(MediaStore);

  /** Route parameter. */
  readonly id = input.required({ transform: numberAttribute });

  protected readonly langs: readonly Lang[] = ['fr', 'en'];
  protected readonly devices = DEVICES;
  protected readonly blockTypes = BLOCK_TYPES;
  protected readonly label = blockLabel;

  protected readonly device = signal<Device>('desktop');
  protected readonly isWide = signal(true);
  protected readonly drawerOpen = signal(false);
  protected readonly pickerAt = signal<number | null>(null);
  protected readonly settingsOpen = signal(false);
  protected readonly historyOpen = signal(false);
  protected readonly versions = signal<PageVersion[]>([]);
  protected readonly toDelete = signal<string | null>(null);
  protected readonly toRestore = signal<number | null>(null);
  protected readonly publishOpen = signal(false);
  protected readonly publishing = signal(false);
  protected readonly publishNotice = signal<string | null>(null);
  protected readonly summary = signal<PublishSummary | null>(null);
  protected readonly message = signal<string | null>(null);
  private confirmedVersion = 0;

  protected readonly previewWidth = computed(() => DEVICE_WIDTHS[this.device()]);
  protected readonly statusLabel = computed(() => STATUS_LABELS[this.store.status()]);
  protected readonly statusClass = computed(() =>
    ['invalid', 'conflict', 'error'].includes(this.store.status())
      ? 'text-destructive'
      : 'text-muted-foreground',
  );
  protected readonly canPublish = computed(
    () =>
      this.store.hasUnpublishedChanges() && !['conflict', 'invalid'].includes(this.store.status()),
  );

  constructor() {
    effect(() => void this.store.load(this.id()));
    void this.media.ensureLoaded();
    const media = typeof matchMedia === 'function' ? matchMedia(WIDE_SCREEN_QUERY) : null;
    if (media) {
      this.isWide.set(media.matches);
      const listener = (event: MediaQueryListEvent) => this.isWide.set(event.matches);
      media.addEventListener('change', listener);
      inject(DestroyRef).onDestroy(() => media.removeEventListener('change', listener));
    }
  }

  /** Used by the route guard: saves what is pending, asks before losing changes. */
  async canLeave(): Promise<boolean> {
    await this.store.flush();
    return (
      !this.store.hasUnsavedChanges() ||
      confirm('Des modifications ne sont pas enregistrées. Quitter quand même ?')
    );
  }

  protected onBeforeUnload(event: BeforeUnloadEvent): void {
    if (this.store.hasUnsavedChanges()) {
      event.preventDefault();
    }
  }

  protected preview = (block: Parameters<typeof toPreviewBlock>[0]) =>
    toPreviewBlock(block, this.store.lang(), this.media.byId());

  protected select(id: string): void {
    this.store.selectedId.set(id);
    this.drawerOpen.set(true);
  }

  protected add(type: BlockType, index: number): void {
    this.pickerAt.set(null);
    this.store.addBlock(type, index);
    this.drawerOpen.set(true);
  }

  protected move(from: number, to: number): void {
    this.store.move(from, to);
  }

  protected drop(event: CdkDragDrop<unknown>): void {
    this.store.move(event.previousIndex, event.currentIndex);
  }

  protected removeBlock(): void {
    const id = this.toDelete();
    this.toDelete.set(null);
    if (id) {
      this.store.remove(id);
    }
  }

  protected async openHistory(): Promise<void> {
    this.versions.set(await this.store.versions());
    this.historyOpen.set(true);
  }

  protected async restore(): Promise<void> {
    const version = this.toRestore();
    this.toRestore.set(null);
    if (version === null) {
      return;
    }
    this.historyOpen.set(false);
    const restored = await this.store.restore(version);
    this.message.set(
      restored
        ? `Version ${version} remise dans le brouillon. Publiez pour la mettre en ligne.`
        : 'La restauration a échoué, réessayez.',
    );
  }

  protected async openPublish(): Promise<void> {
    this.message.set(null);
    this.publishNotice.set(null);
    if (await this.loadSummary()) {
      this.publishOpen.set(true);
    }
  }

  protected async publish(): Promise<void> {
    this.publishing.set(true);
    const outcome = await this.store.publish(this.confirmedVersion);
    this.publishing.set(false);
    if (outcome === 'published') {
      this.publishOpen.set(false);
      this.message.set('Page publiée : les visiteurs voient maintenant cette version.');
      return;
    }
    if (outcome === 'changed') {
      await this.store.load(this.id());
      this.publishNotice.set(
        'La page a été modifiée entre-temps : vérifiez le nouveau résumé avant de publier.',
      );
      if (!(await this.loadSummary())) {
        this.publishOpen.set(false);
      }
      return;
    }
    this.publishNotice.set('La publication a échoué, réessayez.');
  }

  /** Saves what is pending and fetches the summary; false when there is nothing to show. */
  private async loadSummary(): Promise<boolean> {
    const changes = await this.store.changes().catch(() => null);
    const page = this.store.page();
    const content = this.store.content();
    if (!changes || !page || !content) {
      this.message.set('Enregistrez d’abord la page (voir le message ci-dessus), puis publiez.');
      return false;
    }
    if (changes.nothing_to_publish) {
      this.message.set('Rien à publier : le site affiche déjà cette version.');
      return false;
    }
    this.confirmedVersion = changes.draft_version;
    this.summary.set(publishSummary(changes.changes, content, page.published));
    return true;
  }
}
