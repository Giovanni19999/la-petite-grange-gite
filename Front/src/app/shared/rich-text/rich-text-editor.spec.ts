import { TestBed } from '@angular/core/testing';
import { Editor } from '@tiptap/core';

import { RichTextEditor, isAllowedLink, richTextExtensions } from './rich-text-editor';

describe('F023 rich text editor (ADR-0015)', () => {
  it('F023-S08 keeps only the formats the API keeps from pasted content', () => {
    const editor = new Editor({
      element: document.createElement('div'),
      extensions: richTextExtensions(),
      content:
        '<h1>Grand titre</h1><p><strong>Gras</strong> <em>italique</em> <u>souligné</u> ' +
        '<a href="https://example.test">lien</a> <a href="javascript:alert(1)">piège</a></p>' +
        '<table><tr><td>cellule</td></tr></table><img src="x.png"><script>alert(1)</script>' +
        '<ul><li><p>un</p></li></ul><h3>Sous-titre</h3>',
    });
    const html = editor.getHTML();
    editor.destroy();

    expect(html).not.toMatch(/<(table|td|img|script|u|h1)\b/);
    expect(html).not.toContain('javascript:');
    expect(html).not.toContain('alert(1)');
    expect(html).toContain('<strong>Gras</strong>');
    expect(html).toContain('<em>italique</em>');
    expect(html).toContain('href="https://example.test"');
    expect(html).toContain('<ul><li><p>un</p></li></ul>');
    expect(html).toContain('<h3>Sous-titre</h3>');
    expect(html).toContain('cellule');
  });

  it('F023-S08 accepts only http, https, mailto and tel links', () => {
    expect(isAllowedLink('https://example.test')).toBe(true);
    expect(isAllowedLink('mailto:contact@example.test')).toBe(true);
    expect(isAllowedLink('tel:+33200000000')).toBe(true);
    expect(isAllowedLink('javascript:alert(1)')).toBe(false);
    expect(isAllowedLink('data:text/html,x')).toBe(false);
    expect(isAllowedLink('example.test')).toBe(false);
  });

  it('F023-S08 toolbar only offers bold, italic, headings 2 to 4, lists and links', async () => {
    const fixture = TestBed.createComponent(RichTextEditor);
    fixture.componentRef.setInput('label', 'Texte');
    fixture.componentRef.setInput('value', '<p>Bonjour</p>');
    await fixture.whenStable();

    const toolbar = fixture.nativeElement.querySelector('[role="toolbar"]') as HTMLElement;
    const labels = Array.from(toolbar.querySelectorAll('button')).map((button) =>
      button.getAttribute('aria-label'),
    );
    expect(labels).toEqual([
      'Gras',
      'Italique',
      'Titre',
      'Sous-titre',
      'Petit titre',
      'Liste à puces',
      'Liste numérotée',
      'Lien',
    ]);
    expect(fixture.nativeElement.querySelector('[role="textbox"]')?.textContent).toBe('Bonjour');
  });
});
