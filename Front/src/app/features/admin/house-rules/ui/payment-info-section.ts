import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { firstValueFrom } from 'rxjs';

import { RichTextEditor } from '../../../../shared/rich-text/rich-text-editor';
import { ApiErrorBody } from '../../pages/data-access/admin-pages.models';
import { PaymentInfo, PaymentInfoApi } from '../data-access/payment-info.api';

interface PaymentDraft {
  fr: string;
  en: string;
  /** One method per line. */
  methods: string;
}

function draftOf(info: PaymentInfo): PaymentDraft {
  return { ...info.instructions, methods: info.methods.join('\n') };
}

/** The methods typed one per line, without blank lines. */
export function methodsFrom(text: string): string[] {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

/**
 * "Comment payer" (F007, ADR-0020): payment instructions shown to the guests, payment methods
 * and the bank details document attached to the confirmation email. Its own "Valider".
 */
@Component({
  selector: 'app-payment-info-section',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [HlmButtonImports, RichTextEditor],
  template: `
    <section class="flex flex-col gap-4 rounded-lg border p-4" aria-labelledby="payment-title">
      <div>
        <h2 id="payment-title" class="text-lg font-semibold">Comment payer</h2>
        <p class="mt-1 text-sm text-muted-foreground">
          @if (info()?.provider === 'none') {
            Pas de paiement en ligne : la réservation est confirmée tout de suite et le voyageur
            paie l'acompte et le solde comme indiqué ici.
          }
          Ces instructions sont affichées avant la réservation, sur la page de suivi et dans
          l'e-mail de confirmation.
        </p>
      </div>

      @if (draft(); as d) {
        <form class="flex flex-col gap-4" (submit)="$event.preventDefault(); save()">
          <div class="flex flex-col gap-1 text-sm">
            <span>Instructions en français</span>
            <app-rich-text-editor
              label="Instructions de paiement en français"
              [value]="d.fr"
              (valueChange)="patch({ fr: $event })"
            />
          </div>
          <div class="flex flex-col gap-1 text-sm">
            <span>Instructions en anglais (le français est utilisé si vide)</span>
            <app-rich-text-editor
              label="Instructions de paiement en anglais"
              [value]="d.en"
              (valueChange)="patch({ en: $event })"
            />
          </div>
          <label class="flex flex-col gap-1 text-sm">
            Modes de paiement acceptés (un par ligne)
            <textarea
              #methods
              class="min-h-24 max-w-sm rounded-md border border-input p-2"
              name="methods"
              [value]="d.methods"
              (input)="patch({ methods: methods.value })"
            ></textarea>
          </label>
          @if (errors().length > 0) {
            <ul class="list-inside list-disc text-sm text-destructive" role="alert">
              @for (error of errors(); track error) {
                <li>{{ error }}</li>
              }
            </ul>
          }
          <div class="flex flex-wrap items-center gap-3">
            <button hlmBtn type="submit" [disabled]="busy() || !dirty()">Valider</button>
            @if (dirty()) {
              <button hlmBtn variant="outline" type="button" (click)="reset()">Annuler</button>
              <span class="text-xs text-amber-900">Modifications non enregistrées</span>
            } @else if (saved()) {
              <span class="text-sm text-muted-foreground" role="status">
                Instructions enregistrées.
              </span>
            }
          </div>
        </form>

        <div class="flex flex-col gap-2 border-t pt-4 text-sm" data-testid="document">
          <h3 class="font-semibold">RIB joint à l'e-mail de confirmation</h3>
          @if (info()?.document; as document) {
            <p>
              <a class="underline" [href]="documentUrl" target="_blank" rel="noopener">
                {{ document.name }}
              </a>
              — jamais publié sur le site, seulement joint à l'e-mail du voyageur.
            </p>
          } @else {
            <p class="text-muted-foreground">Aucun document : l'e-mail part sans pièce jointe.</p>
          }
          <div class="flex flex-wrap items-center gap-2">
            <label
              class="cursor-pointer rounded-md border border-input px-4 py-2 text-sm font-medium hover:bg-muted has-focus-visible:ring-2 has-focus-visible:ring-ring"
            >
              {{ info()?.document ? 'Remplacer' : 'Ajouter un PDF ou une image' }}
              <input
                #file
                class="sr-only"
                type="file"
                accept="application/pdf,image/jpeg,image/png,image/webp"
                (change)="upload(file)"
              />
            </label>
            @if (info()?.document) {
              <button hlmBtn variant="outline" type="button" (click)="removeDocument()">
                Retirer
              </button>
            }
          </div>
          @if (documentError()) {
            <p class="text-destructive" role="alert">{{ documentError() }}</p>
          }
        </div>
      } @else if (loadError()) {
        <p class="text-destructive" role="alert">Impossible de charger les instructions.</p>
      }
    </section>
  `,
})
export class PaymentInfoSection {
  private readonly api = inject(PaymentInfoApi);

  protected readonly documentUrl = this.api.documentUrl;
  protected readonly info = signal<PaymentInfo | null>(null);
  protected readonly draft = signal<PaymentDraft | null>(null);
  protected readonly loadError = signal(false);
  protected readonly busy = signal(false);
  protected readonly saved = signal(false);
  protected readonly errors = signal<string[]>([]);
  protected readonly documentError = signal('');

  protected readonly dirty = computed(() => {
    const info = this.info();
    const draft = this.draft();
    return (
      info !== null && draft !== null && JSON.stringify(draftOf(info)) !== JSON.stringify(draft)
    );
  });

  constructor() {
    void this.load();
  }

  protected patch(changes: Partial<PaymentDraft>): void {
    this.draft.update((draft) => (draft ? { ...draft, ...changes } : draft));
    this.saved.set(false);
  }

  protected reset(): void {
    const info = this.info();
    if (info) {
      this.draft.set(draftOf(info));
    }
    this.errors.set([]);
  }

  protected async save(): Promise<void> {
    const draft = this.draft();
    if (!draft) {
      return;
    }
    this.busy.set(true);
    this.errors.set([]);
    try {
      await firstValueFrom(
        this.api.save({ fr: draft.fr, en: draft.en }, methodsFrom(draft.methods)),
      );
      await this.load();
      this.saved.set(true);
    } catch (error) {
      this.errors.set(messages(error));
    } finally {
      this.busy.set(false);
    }
  }

  protected async upload(input: HTMLInputElement): Promise<void> {
    const file = input.files?.[0];
    input.value = '';
    if (!file) {
      return;
    }
    this.documentError.set('');
    try {
      await firstValueFrom(this.api.uploadDocument(file));
      await this.load(true);
    } catch (error) {
      this.documentError.set(messages(error).join(' '));
    }
  }

  protected async removeDocument(): Promise<void> {
    try {
      await firstValueFrom(this.api.removeDocument());
      await this.load(true);
    } catch (error) {
      this.documentError.set(messages(error).join(' '));
    }
  }

  /** `keepDraft`: only the document changed, the instructions being typed stay. */
  private async load(keepDraft = false): Promise<void> {
    try {
      const info = await firstValueFrom(this.api.info());
      const typing = keepDraft && this.dirty() ? this.draft() : null;
      this.info.set(info);
      this.draft.set(typing ?? draftOf(info));
      this.loadError.set(false);
    } catch {
      this.loadError.set(true);
    }
  }
}

function messages(error: unknown): string[] {
  const body = error instanceof HttpErrorResponse ? (error.error as ApiErrorBody | null) : null;
  const details = body?.error?.details ?? [];
  if (details.length > 0) {
    return details;
  }
  return [body?.error?.message ?? 'Une erreur est survenue, réessayez plus tard.'];
}
