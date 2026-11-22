import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  effect,
  inject,
  input,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { Editor, Extensions } from '@tiptap/core';
import Bold from '@tiptap/extension-bold';
import Document from '@tiptap/extension-document';
import HardBreak from '@tiptap/extension-hard-break';
import Heading from '@tiptap/extension-heading';
import Italic from '@tiptap/extension-italic';
import Link from '@tiptap/extension-link';
import { BulletList, ListItem, OrderedList } from '@tiptap/extension-list';
import Paragraph from '@tiptap/extension-paragraph';
import Text from '@tiptap/extension-text';
import { UndoRedo } from '@tiptap/extensions';

/** Link schemes kept by the API sanitiser (F002). */
export const ALLOWED_LINK_PROTOCOLS = ['http', 'https', 'mailto', 'tel'] as const;

/** Whether `href` uses a scheme the API keeps. */
export function isAllowedLink(href: string): boolean {
  const scheme = /^([a-z][a-z0-9+.-]*):/i.exec(href.trim())?.[1]?.toLowerCase();
  return scheme !== undefined && (ALLOWED_LINK_PROTOCOLS as readonly string[]).includes(scheme);
}

/**
 * Only the formats the API keeps (ADR-0015): paragraphs, line breaks, bold, italic, lists,
 * headings 2 to 4 and links. Anything else typed or pasted is dropped by the schema.
 */
export function richTextExtensions(): Extensions {
  return [
    Document,
    Paragraph,
    Text,
    HardBreak,
    Bold,
    Italic,
    BulletList,
    OrderedList,
    ListItem,
    Heading.configure({ levels: [2, 3, 4] }),
    Link.configure({
      openOnClick: false,
      autolink: true,
      defaultProtocol: 'https',
      protocols: ['mailto', 'tel'],
      isAllowedUri: (url) => isAllowedLink(url),
      HTMLAttributes: { rel: 'noopener noreferrer', target: null },
    }),
    UndoRedo,
  ];
}

type Command =
  'bold' | 'italic' | 'heading2' | 'heading3' | 'heading4' | 'bulletList' | 'orderedList';

interface ToolbarButton {
  command: Command;
  label: string;
  text: string;
}

const BUTTONS: readonly ToolbarButton[] = [
  { command: 'bold', label: 'Gras', text: 'G' },
  { command: 'italic', label: 'Italique', text: 'I' },
  { command: 'heading2', label: 'Titre', text: 'T1' },
  { command: 'heading3', label: 'Sous-titre', text: 'T2' },
  { command: 'heading4', label: 'Petit titre', text: 'T3' },
  { command: 'bulletList', label: 'Liste à puces', text: '•' },
  { command: 'orderedList', label: 'Liste numérotée', text: '1.' },
];

/**
 * Rich text field of the admin (ADR-0015): Tiptap behind our own component. The editor is
 * created in the browser only, never during server-side rendering.
 */
@Component({
  selector: 'app-rich-text-editor',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="rounded-md border border-input focus-within:ring-2 focus-within:ring-ring/50">
      <div
        class="flex flex-wrap gap-1 border-b border-input p-1"
        role="toolbar"
        [attr.aria-label]="'Mise en forme : ' + label()"
      >
        @for (button of buttons; track button.command) {
          <button
            type="button"
            class="min-w-9 rounded px-2 py-1 text-sm hover:bg-muted aria-pressed:bg-primary aria-pressed:text-primary-foreground"
            [class.font-bold]="button.command === 'bold'"
            [class.italic]="button.command === 'italic'"
            [attr.aria-label]="button.label"
            [title]="button.label"
            [attr.aria-pressed]="active()[button.command] ?? false"
            (click)="run(button.command)"
          >
            {{ button.text }}
          </button>
        }
        <button
          type="button"
          class="rounded px-2 py-1 text-sm underline hover:bg-muted aria-pressed:bg-primary aria-pressed:text-primary-foreground"
          aria-label="Lien"
          title="Lien"
          [attr.aria-pressed]="active()['link'] ?? false"
          (click)="toggleLinkForm()"
        >
          Lien
        </button>
      </div>
      @if (linkFormOpen()) {
        <div class="flex flex-wrap items-center gap-2 border-b border-input p-2">
          <label class="flex min-w-0 flex-1 items-center gap-2 text-sm">
            Adresse
            <input
              #linkInput
              class="h-8 min-w-0 flex-1 rounded border border-input px-2"
              type="url"
              placeholder="https://… ou mailto:… ou tel:…"
              [value]="linkHref()"
              (input)="linkHref.set(linkInput.value)"
              (keydown.enter)="$event.preventDefault(); applyLink()"
              (keydown.escape)="linkFormOpen.set(false)"
            />
          </label>
          <button
            type="button"
            class="rounded bg-primary px-2 py-1 text-sm text-primary-foreground"
            (click)="applyLink()"
          >
            OK
          </button>
          <button
            type="button"
            class="rounded px-2 py-1 text-sm hover:bg-muted"
            (click)="removeLink()"
          >
            Retirer le lien
          </button>
          @if (linkError()) {
            <p class="w-full text-sm text-destructive" role="alert">
              Adresse non acceptée : utilisez http, https, mailto ou tel.
            </p>
          }
        </div>
      }
      <div
        #host
        class="rich-text min-h-32 px-3 py-2 [&_.ProseMirror]:min-h-28 [&_.ProseMirror]:outline-none"
      ></div>
    </div>
  `,
})
export class RichTextEditor {
  /** Accessible name of the field. */
  readonly label = input.required<string>();
  /** HTML content; changing it from outside replaces the editor content. */
  readonly value = input('');
  readonly valueChange = output<string>();

  protected readonly buttons = BUTTONS;
  protected readonly active = signal<Partial<Record<Command | 'link', boolean>>>({});
  protected readonly linkFormOpen = signal(false);
  protected readonly linkHref = signal('');
  protected readonly linkError = signal(false);

  private readonly host = viewChild.required<ElementRef<HTMLElement>>('host');
  private editor: Editor | null = null;

  constructor() {
    afterNextRender(() => {
      this.editor = new Editor({
        element: this.host().nativeElement,
        extensions: richTextExtensions(),
        content: this.value(),
        editorProps: {
          attributes: {
            role: 'textbox',
            'aria-multiline': 'true',
            'aria-label': this.label(),
          },
        },
        onUpdate: ({ editor }) => this.valueChange.emit(normalize(editor.getHTML())),
        onTransaction: ({ editor }) => this.refreshActive(editor),
      });
    });
    effect(() => {
      const value = this.value();
      const editor = this.editor;
      if (editor && normalize(editor.getHTML()) !== value) {
        editor.commands.setContent(value, { emitUpdate: false });
      }
    });
    inject(DestroyRef).onDestroy(() => this.editor?.destroy());
  }

  /** Inserts plain text at the cursor (a template variable, F007). */
  insertText(text: string): void {
    this.editor?.chain().focus().insertContent(text).run();
  }

  protected run(command: Command): void {
    const chain = this.editor?.chain().focus();
    if (!chain) {
      return;
    }
    const commands: Record<Command, () => boolean> = {
      bold: () => chain.toggleBold().run(),
      italic: () => chain.toggleItalic().run(),
      heading2: () => chain.toggleHeading({ level: 2 }).run(),
      heading3: () => chain.toggleHeading({ level: 3 }).run(),
      heading4: () => chain.toggleHeading({ level: 4 }).run(),
      bulletList: () => chain.toggleBulletList().run(),
      orderedList: () => chain.toggleOrderedList().run(),
    };
    commands[command]();
  }

  protected toggleLinkForm(): void {
    const href = (this.editor?.getAttributes('link')['href'] as string | undefined) ?? '';
    this.linkHref.set(href);
    this.linkError.set(false);
    this.linkFormOpen.update((open) => !open);
  }

  protected applyLink(): void {
    const href = this.linkHref().trim();
    if (!isAllowedLink(href)) {
      this.linkError.set(true);
      return;
    }
    this.editor?.chain().focus().extendMarkRange('link').setLink({ href }).run();
    this.linkFormOpen.set(false);
  }

  protected removeLink(): void {
    this.editor?.chain().focus().extendMarkRange('link').unsetLink().run();
    this.linkFormOpen.set(false);
  }

  private refreshActive(editor: Editor): void {
    this.active.set({
      bold: editor.isActive('bold'),
      italic: editor.isActive('italic'),
      heading2: editor.isActive('heading', { level: 2 }),
      heading3: editor.isActive('heading', { level: 3 }),
      heading4: editor.isActive('heading', { level: 4 }),
      bulletList: editor.isActive('bulletList'),
      orderedList: editor.isActive('orderedList'),
      link: editor.isActive('link'),
    });
  }
}

/** An editor with no text yields `<p></p>`: store it as an empty text. */
function normalize(html: string): string {
  return html === '<p></p>' ? '' : html;
}
