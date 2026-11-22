import { HttpErrorResponse } from '@angular/common/http';
import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { HlmButtonImports } from '@spartan-ng/helm/button';
import { firstValueFrom } from 'rxjs';

import { RichTextEditor } from '../../../shared/rich-text/rich-text-editor';
import { ApiErrorBody } from '../pages/data-access/admin-pages.models';
import {
  MessageTemplate,
  Preview,
  TemplateChannel,
  TemplateKey,
  TemplateLang,
  TemplatesApi,
} from './data-access/templates.api';
import { smsCount } from './data-access/sms-count';

interface Draft {
  subject: string;
  body: string;
}

/**
 * "Modèles de messages" (F007, ADR-0021): the emails and SMS sent when a booking is made or its
 * deposit is late, in French and English, with variables inserted by buttons.
 */
@Component({
  selector: 'app-templates-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink, HlmButtonImports, RichTextEditor],
  template: `
    <section class="mx-auto flex max-w-4xl flex-col gap-6 px-4 py-8">
      <div>
        <a routerLink="/admin" class="text-sm text-muted-foreground hover:underline">
          ← Espace administrateur
        </a>
        <h1 class="text-2xl font-semibold">Modèles de messages</h1>
        <p class="mt-1 text-sm text-muted-foreground">
          Les e-mails et SMS envoyés automatiquement. Un texte vide n'est pas envoyé ; en anglais,
          un texte vide est remplacé par le français.
        </p>
      </div>
      @if (loadError()) {
        <p class="text-destructive" role="alert">Impossible de charger les modèles.</p>
      }

      <div class="flex flex-wrap gap-2" role="tablist" aria-label="Modèles">
        @for (template of templates(); track template.kind) {
          <button
            hlmBtn
            type="button"
            role="tab"
            [variant]="template.kind === key()?.kind ? 'default' : 'outline'"
            [attr.aria-selected]="template.kind === key()?.kind"
            (click)="open(template.kind, key()?.channel ?? 'email', 'fr')"
          >
            {{ template.label }}
            <span class="ml-1 text-xs opacity-80">
              ({{ template.recipient === 'guest' ? 'voyageur' : 'propriétaires' }})
            </span>
          </button>
        }
      </div>

      @if (current(); as t) {
        @if (key(); as k) {
          <div class="flex flex-wrap gap-4 text-sm">
            <div class="flex gap-1" role="group" aria-label="Canal">
              @for (channel of channels; track channel.value) {
                <button
                  hlmBtn
                  size="sm"
                  type="button"
                  [variant]="channel.value === k.channel ? 'default' : 'outline'"
                  [attr.aria-pressed]="channel.value === k.channel"
                  (click)="open(k.kind, channel.value, k.lang)"
                >
                  {{ channel.label }}
                </button>
              }
            </div>
            @if (t.recipient === 'guest') {
              <div class="flex gap-1" role="group" aria-label="Langue">
                @for (lang of langs; track lang.value) {
                  <button
                    hlmBtn
                    size="sm"
                    type="button"
                    [variant]="lang.value === k.lang ? 'default' : 'outline'"
                    [attr.aria-pressed]="lang.value === k.lang"
                    (click)="open(k.kind, k.channel, lang.value)"
                  >
                    {{ lang.label }}
                  </button>
                }
              </div>
            }
          </div>

          @if (draft(); as d) {
            <form class="flex flex-col gap-4" (submit)="$event.preventDefault(); save()">
              <div class="flex flex-wrap gap-1" aria-label="Insérer une variable" role="group">
                @for (variable of t.variables; track variable.name) {
                  <button
                    type="button"
                    class="rounded border px-2 py-1 text-xs hover:bg-muted"
                    [title]="code(variable.name)"
                    (click)="insert(variable.name)"
                  >
                    {{ variable.label }}
                  </button>
                }
              </div>

              @if (k.channel === 'email') {
                <label class="flex flex-col gap-1 text-sm">
                  Objet
                  <input
                    #subject
                    class="h-9 rounded-md border border-input px-2"
                    name="subject"
                    [value]="d.subject"
                    (focus)="target.set('subject')"
                    (input)="patch({ subject: subject.value })"
                  />
                </label>
                <div class="flex flex-col gap-1 text-sm" (focusin)="target.set('body')">
                  <span>Texte</span>
                  <app-rich-text-editor
                    #editor
                    label="Texte de l'e-mail"
                    [value]="d.body"
                    (valueChange)="patch({ body: $event })"
                  />
                </div>
              } @else {
                <label class="flex flex-col gap-1 text-sm">
                  Texte du SMS
                  <textarea
                    #sms
                    class="min-h-28 rounded-md border border-input p-2"
                    name="sms"
                    [value]="d.body"
                    (input)="patch({ body: sms.value })"
                  ></textarea>
                </label>
                <p
                  class="text-sm"
                  data-testid="sms-count"
                  [class.text-amber-900]="count().segments > 2"
                >
                  {{ count().length }} caractères · {{ count().segments }} SMS{{
                    count().unicode ? ' (caractères spéciaux : 70 par SMS)' : ''
                  }}
                  @if (count().segments > 2) {
                    — plus de 2 SMS : pensez à raccourcir (les variables allongent le texte).
                  }
                </p>
              }

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
                  <span class="text-sm text-muted-foreground" role="status"
                    >Modèle enregistré.</span
                  >
                }
                <button hlmBtn variant="outline" type="button" (click)="showPreview()">
                  Aperçu
                </button>
                @if (changed()) {
                  <button hlmBtn variant="ghost" type="button" (click)="restore()">
                    Rétablir le texte d'origine
                  </button>
                }
              </div>
            </form>
          }

          @if (preview(); as p) {
            <section class="flex flex-col gap-2 rounded-lg border p-4" data-testid="preview">
              <h2 class="font-semibold">Aperçu avec une réservation d'exemple</h2>
              @if (p.subject) {
                <p class="text-sm"><span class="font-medium">Objet :</span> {{ p.subject }}</p>
              }
              @if (previewHtml(); as html) {
                <iframe
                  class="h-[32rem] w-full rounded border"
                  title="Aperçu de l'e-mail"
                  sandbox=""
                  [srcdoc]="html"
                ></iframe>
              } @else {
                <p class="rounded bg-muted p-3 text-sm whitespace-pre-line">{{ p.text }}</p>
              }
            </section>
          }
        }
      }
    </section>
  `,
})
export class TemplatesPage {
  private readonly api = inject(TemplatesApi);
  private readonly sanitizer = inject(DomSanitizer);
  private readonly editor = viewChild<RichTextEditor>('editor');

  protected readonly channels: readonly { value: TemplateChannel; label: string }[] = [
    { value: 'email', label: 'E-mail' },
    { value: 'sms', label: 'SMS' },
  ];
  protected readonly langs: readonly { value: TemplateLang; label: string }[] = [
    { value: 'fr', label: 'Français' },
    { value: 'en', label: 'Anglais' },
  ];

  protected readonly templates = signal<MessageTemplate[]>([]);
  protected readonly key = signal<TemplateKey | null>(null);
  protected readonly draft = signal<Draft | null>(null);
  protected readonly loadError = signal(false);
  protected readonly busy = signal(false);
  protected readonly saved = signal(false);
  protected readonly errors = signal<string[]>([]);
  protected readonly preview = signal<Preview | null>(null);
  /** Where a variable button inserts: the subject or the body. */
  protected readonly target = signal<'subject' | 'body'>('body');

  protected readonly current = computed(() =>
    this.templates().find((template) => template.kind === this.key()?.kind),
  );
  private readonly text = computed(() => {
    const key = this.key();
    return this.current()
      ?.channels.find((channel) => channel.channel === key?.channel)
      ?.texts.find((text) => text.lang === key?.lang);
  });
  protected readonly changed = computed(() => this.text()?.changed ?? false);
  protected readonly dirty = computed(() => {
    const text = this.text();
    const draft = this.draft();
    return !!text && !!draft && (text.subject !== draft.subject || text.body !== draft.body);
  });
  protected readonly count = computed(() => smsCount(this.draft()?.body ?? ''));
  protected readonly previewHtml = computed<SafeHtml | null>(() => {
    const html = this.preview()?.html;
    // Our own API's output, shown in a sandboxed frame without scripts.
    return html ? this.sanitizer.bypassSecurityTrustHtml(html) : null;
  });

  constructor() {
    void this.load();
  }

  protected open(kind: string, channel: TemplateChannel, lang: TemplateLang): void {
    if (this.dirty() && !confirm('Abandonner les modifications non enregistrées ?')) {
      return;
    }
    const template = this.templates().find((candidate) => candidate.kind === kind);
    const usable: TemplateLang = template?.recipient === 'owners' ? 'fr' : lang;
    this.key.set({ kind, channel, lang: usable });
    this.target.set('body');
    this.preview.set(null);
    this.errors.set([]);
    this.saved.set(false);
    this.reset();
  }

  protected patch(changes: Partial<Draft>): void {
    this.draft.update((draft) => (draft ? { ...draft, ...changes } : draft));
    this.saved.set(false);
  }

  protected reset(): void {
    const text = this.text();
    this.draft.set(text ? { subject: text.subject, body: text.body } : null);
    this.errors.set([]);
  }

  /** How a variable is written in a template. */
  protected code(name: string): string {
    return `{{ ${name} }}`;
  }

  protected insert(name: string): void {
    const variable = this.code(name);
    const key = this.key();
    const draft = this.draft();
    if (!key || !draft) {
      return;
    }
    if (key.channel === 'sms') {
      this.patch({
        body: draft.body + (draft.body.endsWith(' ') || !draft.body ? '' : ' ') + variable,
      });
    } else if (this.target() === 'subject') {
      this.patch({ subject: `${draft.subject} ${variable}`.trim() });
    } else {
      this.editor()?.insertText(variable);
    }
  }

  protected async save(): Promise<void> {
    const key = this.key();
    const draft = this.draft();
    if (!key || !draft) {
      return;
    }
    this.busy.set(true);
    this.errors.set([]);
    try {
      await firstValueFrom(this.api.save(key, draft.subject, draft.body));
      await this.load();
      this.reset();
      this.saved.set(true);
    } catch (error) {
      this.errors.set(messages(error));
    } finally {
      this.busy.set(false);
    }
  }

  protected async restore(): Promise<void> {
    const key = this.key();
    if (!key || !confirm("Remettre le texte d'origine de ce modèle ?")) {
      return;
    }
    try {
      await firstValueFrom(this.api.restore(key));
      await this.load();
      this.reset();
    } catch (error) {
      this.errors.set(messages(error));
    }
  }

  protected async showPreview(): Promise<void> {
    const key = this.key();
    const draft = this.draft();
    if (!key || !draft) {
      return;
    }
    this.errors.set([]);
    try {
      this.preview.set(await firstValueFrom(this.api.preview(key, draft.subject, draft.body)));
    } catch (error) {
      this.preview.set(null);
      this.errors.set(messages(error));
    }
  }

  private async load(): Promise<void> {
    try {
      this.templates.set(await firstValueFrom(this.api.list()));
      this.loadError.set(false);
      if (!this.key() && this.templates().length > 0) {
        this.open(this.templates()[0].kind, 'email', 'fr');
      }
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
