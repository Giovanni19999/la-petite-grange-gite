import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { HlmDialogImports } from '@spartan-ng/helm/dialog';
import { HlmInputImports } from '@spartan-ng/helm/input';
import { HlmLabelImports } from '@spartan-ng/helm/label';
import { HlmTextareaImports } from '@spartan-ng/helm/textarea';

import { Lang } from '../../../public/pages/data-access/pages.models';
import { Localized, PageSettings } from '../data-access/admin-pages.models';

/** Settings of the page (F023): address, title and description per language, menu. */
@Component({
  selector: 'app-page-settings-dialog',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [HlmDialogImports, HlmInputImports, HlmLabelImports, HlmTextareaImports],
  template: `
    <hlm-dialog [state]="open() ? 'open' : 'closed'" (closed)="closed.emit()">
      <hlm-dialog-content *hlmDialogPortal="let ctx" class="max-h-[90vh] overflow-y-auto">
        <hlm-dialog-header>
          <h2 hlmDialogTitle>Réglages de la page</h2>
          <p hlmDialogDescription>Enregistrés automatiquement, publiés avec la page.</p>
        </hlm-dialog-header>

        @let s = settings();
        <div class="flex flex-col gap-6">
          @for (lang of langs; track lang) {
            <fieldset class="flex flex-col gap-3">
              <legend class="mb-1 font-medium">{{ lang === 'fr' ? 'Français' : 'Anglais' }}</legend>
              <label hlmLabel class="flex flex-col items-start gap-2">
                Titre
                <input
                  #title
                  hlmInput
                  class="w-full"
                  [value]="s.title[lang]"
                  [placeholder]="lang === 'en' ? s.title.fr : ''"
                  (input)="patch({ title: text(s.title, lang, title.value) })"
                />
              </label>
              <label hlmLabel class="flex flex-col items-start gap-2">
                Adresse de la page
                <span class="flex w-full items-center gap-1 text-muted-foreground">
                  {{ lang === 'en' ? '/en/' : '/' }}
                  <input
                    #slug
                    hlmInput
                    class="w-full"
                    [value]="s.slug[lang]"
                    [disabled]="isHome()"
                    [attr.aria-invalid]="slugError() ? 'true' : null"
                    (input)="patch({ slug: text(s.slug, lang, slug.value.trim()) })"
                  />
                </span>
              </label>
              <label hlmLabel class="flex flex-col items-start gap-2">
                Description pour les moteurs de recherche
                <textarea
                  #description
                  hlmTextarea
                  class="w-full"
                  rows="2"
                  [value]="s.meta_description[lang]"
                  (input)="
                    patch({ meta_description: text(s.meta_description, lang, description.value) })
                  "
                ></textarea>
              </label>
            </fieldset>
          }
          @if (isHome()) {
            <p class="text-sm text-muted-foreground">
              La page d’accueil garde les adresses « / » et « /en ».
            </p>
          }
          @if (slugError(); as message) {
            <p class="text-sm text-destructive" role="alert" data-testid="slug-error">
              {{ message }}
            </p>
          }

          <fieldset class="flex flex-col gap-3">
            <legend class="mb-1 font-medium">Menu</legend>
            <label class="flex items-center gap-2 text-sm">
              <input
                #inMenu
                type="checkbox"
                class="size-4 accent-primary"
                [checked]="s.in_menu"
                [disabled]="isHome()"
                (change)="patch({ in_menu: inMenu.checked })"
              />
              Afficher dans le menu
            </label>
            <label hlmLabel class="flex flex-col items-start gap-2">
              Position dans le menu (les plus petits nombres d’abord)
              <input
                #order
                hlmInput
                class="w-24"
                type="number"
                min="0"
                [value]="s.menu_order"
                [disabled]="isHome()"
                (input)="patch({ menu_order: toOrder(order.value) })"
              />
            </label>
          </fieldset>
        </div>
      </hlm-dialog-content>
    </hlm-dialog>
  `,
})
export class PageSettingsDialog {
  readonly open = input(false);
  readonly settings = input.required<PageSettings>();
  readonly isHome = input(false);
  readonly slugError = input<string | null>(null);

  readonly changed = output<PageSettings>();
  readonly closed = output<void>();

  protected readonly langs: readonly Lang[] = ['fr', 'en'];

  protected patch(changes: Partial<PageSettings>): void {
    this.changed.emit({ ...this.settings(), ...changes });
  }

  protected text(current: Localized, lang: Lang, value: string): Localized {
    return { ...current, [lang]: value };
  }

  protected toOrder(value: string): number {
    const order = Math.trunc(Number(value));
    return Number.isFinite(order) && order >= 0 ? order : 0;
  }
}
