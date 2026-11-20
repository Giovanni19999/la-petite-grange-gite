import { DatePipe } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { HlmInputImports } from '@spartan-ng/helm/input';
import { HlmLabelImports } from '@spartan-ng/helm/label';
import { firstValueFrom } from 'rxjs';

import { ConfirmDialog } from '../../../../shared/confirm/confirm-dialog';
import { AdminPagesApi } from '../data-access/admin-pages.api';
import { AdminPageSummary, ApiErrorBody } from '../data-access/admin-pages.models';
import { newPageContent, slugify } from '../data-access/page-content';

/** Pages of the site (F023): list, create, delete; each page opens in the editor. */
@Component({
  selector: 'app-pages-list-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    DatePipe,
    FormsModule,
    RouterLink,
    HlmButtonImports,
    HlmInputImports,
    HlmLabelImports,
    ConfirmDialog,
  ],
  template: `
    <section class="mx-auto flex max-w-3xl flex-col gap-6 px-4 py-8">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <a routerLink="/admin" class="text-sm text-muted-foreground hover:underline">
            ← Espace administrateur
          </a>
          <h1 class="text-2xl font-semibold">Pages du site</h1>
        </div>
        <button hlmBtn type="button" (click)="creating.set(!creating())">Nouvelle page</button>
      </div>

      @if (creating()) {
        <form
          class="flex flex-col gap-3 rounded-lg border p-4"
          aria-label="Nouvelle page"
          (ngSubmit)="create()"
        >
          <label hlmLabel class="flex flex-col items-start gap-2">
            Titre (français)
            <input
              hlmInput
              class="w-full"
              name="title"
              required
              [ngModel]="newTitle()"
              (ngModelChange)="onTitle($event)"
            />
          </label>
          <label hlmLabel class="flex flex-col items-start gap-2">
            Adresse de la page
            <span class="flex w-full items-center gap-1 text-muted-foreground">
              /<input
                hlmInput
                class="w-full"
                name="slug"
                required
                [ngModel]="newSlug()"
                (ngModelChange)="onSlug($event)"
              />
            </span>
          </label>
          @if (createError(); as message) {
            <p class="text-sm text-destructive" role="alert">{{ message }}</p>
          }
          <div class="flex gap-2">
            <button hlmBtn type="submit" [disabled]="busy()">Créer la page</button>
            <button hlmBtn variant="outline" type="button" (click)="creating.set(false)">
              Annuler
            </button>
          </div>
        </form>
      }

      @if (loadError()) {
        <p class="text-destructive" role="alert">
          Impossible de charger les pages, réessayez plus tard.
        </p>
      }

      <ul class="flex flex-col divide-y rounded-lg border" aria-label="Pages">
        @for (page of pages(); track page.id) {
          <li class="flex flex-wrap items-center gap-3 p-4" data-testid="page-row">
            <div class="flex min-w-0 flex-1 flex-col">
              <a class="font-medium hover:underline" [routerLink]="['/admin/pages', page.id]">{{
                page.title
              }}</a>
              <span class="text-sm text-muted-foreground">
                {{ page.is_home ? '/' : '/' + page.slug }}
                @if (page.published_at) {
                  · publiée le {{ page.published_at | date: 'dd/MM/yyyy à HH:mm' }}
                }
              </span>
            </div>
            @if (page.is_home) {
              <span class="rounded-full bg-muted px-2 py-0.5 text-xs">Accueil</span>
            }
            @if (page.unpublished_changes) {
              <span
                class="rounded-full bg-primary/10 px-2 py-0.5 text-xs text-primary"
                data-testid="not-published"
                >{{ page.published_at ? 'Modifications non publiées' : 'Jamais publiée' }}</span
              >
            }
            <a hlmBtn variant="outline" size="sm" [routerLink]="['/admin/pages', page.id]">
              Modifier
            </a>
            @if (!page.is_home) {
              <button
                hlmBtn
                variant="ghost"
                size="sm"
                type="button"
                [attr.aria-label]="'Supprimer la page ' + page.title"
                (click)="toDelete.set(page)"
              >
                Supprimer
              </button>
            }
          </li>
        }
      </ul>
    </section>

    <app-confirm-dialog
      [open]="toDelete() !== null"
      [title]="'Supprimer la page « ' + (toDelete()?.title ?? '') + ' » ?'"
      description="La page et son historique seront supprimés. Elle disparaîtra du site."
      confirmLabel="Supprimer"
      [destructive]="true"
      (confirmed)="remove()"
      (cancelled)="toDelete.set(null)"
    />
  `,
})
export class PagesListPage {
  private readonly api = inject(AdminPagesApi);
  private readonly router = inject(Router);

  protected readonly pages = signal<AdminPageSummary[]>([]);
  protected readonly loadError = signal(false);
  protected readonly creating = signal(false);
  protected readonly newTitle = signal('');
  protected readonly newSlug = signal('');
  protected readonly createError = signal<string | null>(null);
  protected readonly busy = signal(false);
  protected readonly toDelete = signal<AdminPageSummary | null>(null);
  private slugEdited = false;

  constructor() {
    void this.reload();
  }

  protected onTitle(title: string): void {
    this.newTitle.set(title);
    if (!this.slugEdited || this.newSlug() === '') {
      this.newSlug.set(slugify(title));
    }
  }

  protected onSlug(slug: string): void {
    this.slugEdited = true;
    this.newSlug.set(slug);
  }

  protected async create(): Promise<void> {
    const title = this.newTitle().trim();
    const slug = this.newSlug().trim();
    if (!title || !slug) {
      this.createError.set('Renseignez le titre et l’adresse de la page.');
      return;
    }
    this.busy.set(true);
    this.createError.set(null);
    try {
      const { id } = await firstValueFrom(
        this.api.create(newPageContent(title, slug, this.pages().length)),
      );
      await this.router.navigate(['/admin/pages', id]);
    } catch (error) {
      this.createError.set(errorMessage(error));
    } finally {
      this.busy.set(false);
    }
  }

  protected async remove(): Promise<void> {
    const page = this.toDelete();
    this.toDelete.set(null);
    if (!page) {
      return;
    }
    try {
      await firstValueFrom(this.api.delete(page.id));
    } finally {
      await this.reload();
    }
  }

  private async reload(): Promise<void> {
    try {
      this.pages.set(await firstValueFrom(this.api.list()));
      this.loadError.set(false);
    } catch {
      this.loadError.set(true);
    }
  }
}

/** Message of an API error, in French. */
function errorMessage(error: unknown): string {
  const body = error instanceof HttpErrorResponse ? (error.error as ApiErrorBody | null) : null;
  const details = body?.error?.details ?? [];
  if (details.length > 0) {
    return details.join(' ; ');
  }
  return body?.error?.message ?? 'Une erreur est survenue, réessayez plus tard.';
}
