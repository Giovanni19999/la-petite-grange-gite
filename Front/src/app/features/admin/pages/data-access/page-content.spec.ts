import { DraftHeroBlock, PageContent } from './admin-pages.models';
import {
  blockIds,
  duplicateBlock,
  insertBlock,
  moveBlock,
  newBlock,
  newPageContent,
  publishSummary,
  removeBlock,
  sameContent,
  slugify,
  toPreviewBlock,
} from './page-content';

function hero(title: { fr: string; en: string }): DraftHeroBlock {
  return { ...(newBlock('hero', new Set()) as DraftHeroBlock), id: 'hero', title };
}

function page(): PageContent {
  return {
    ...newPageContent('Accueil', 'accueil', 0),
    blocks: [
      hero({ fr: 'La Petite Grange', en: '' }),
      { ...newBlock('text', new Set()), id: 'intro' },
      { ...newBlock('columns', new Set()), id: 'atouts' },
    ],
  };
}

const ids = (content: PageContent) => content.blocks.map((block) => block.id);

describe('F023 page content', () => {
  it('F023-S05 adds, duplicates, moves and deletes blocks', () => {
    let content = page();
    const text = newBlock('text', blockIds(content));
    content = insertBlock(content, 1, text);
    expect(ids(content)).toEqual(['hero', text.id, 'intro', 'atouts']);

    content = duplicateBlock(content, text.id);
    const copy = content.blocks[2];
    expect(copy.id).not.toBe(text.id);
    expect({ ...copy, id: text.id }).toEqual(text);

    content = moveBlock(content, 2, 1);
    expect(ids(content)).toEqual(['hero', copy.id, text.id, 'intro', 'atouts']);

    content = removeBlock(content, copy.id);
    expect(ids(content)).toEqual(['hero', text.id, 'intro', 'atouts']);
  });

  it('F023-S05 new block ids are unique and accepted by the API', () => {
    const taken = new Set<string>();
    for (let i = 0; i < 50; i++) {
      const id = newBlock('text_image', taken).id;
      expect(id).toMatch(/^[A-Za-z0-9_-]{1,40}$/);
      expect(taken.has(id)).toBe(false);
      taken.add(id);
    }
  });

  it('F023-S06 moves a block to another position', () => {
    expect(ids(moveBlock(page(), 2, 0))).toEqual(['atouts', 'hero', 'intro']);
    expect(ids(moveBlock(page(), 0, 9))).toEqual(['intro', 'atouts', 'hero']);
  });

  it('F023-S07 preview shows the French text when the English one is empty, marked as fallback', () => {
    const block = hero({ fr: 'La Petite Grange', en: '' });

    const english = toPreviewBlock(block, 'en');
    expect(english.type === 'hero' && english.title).toBe('La Petite Grange');
    expect(english.fallback).toEqual(['title']);

    const french = toPreviewBlock(block, 'fr');
    expect(french.fallback).toBeUndefined();

    const translated = toPreviewBlock({ ...block, title: { fr: 'Grange', en: 'Barn' } }, 'en');
    expect(translated.type === 'hero' && translated.title).toBe('Barn');
    expect(translated.fallback).toBeUndefined();
  });

  it('F023-S12 summary of changes names blocks by type', () => {
    const draft = page();
    const summary = publishSummary(
      {
        settings_changed: false,
        blocks_added: ['intro'],
        blocks_changed: ['hero'],
        blocks_removed: ['old'],
        blocks_moved: true,
        languages: ['fr', 'en'],
      },
      draft,
      { ...draft, blocks: [{ ...newBlock('gallery', new Set()), id: 'old' }] },
    );
    expect(summary.lines).toEqual([
      { label: 'Bloc ajouté', items: ['Texte'] },
      { label: 'Bloc modifié', items: ['Bandeau'] },
      { label: 'Bloc supprimé', items: ['Galerie'] },
      { label: 'Ordre des blocs modifié', items: [] },
    ]);
    expect(summary.languages).toEqual(['fr', 'en']);
  });

  it('F023-S02 new page: slug from the title, same slug in English, not in the menu', () => {
    expect(slugify('  Activités & loisirs à côté ')).toBe('activites-loisirs-a-cote');
    const content = newPageContent('Activités', 'activites', 3);
    expect(content.settings.slug).toEqual({ fr: 'activites', en: 'activites' });
    expect(content.settings.title).toEqual({ fr: 'Activités', en: '' });
    expect(content.settings.in_menu).toBe(false);
    expect(content.blocks).toEqual([]);
  });

  it('compares contents whatever the order of the keys', () => {
    const content = page();
    const reordered = JSON.parse(
      JSON.stringify({ blocks: content.blocks, settings: content.settings }),
    );
    expect(sameContent(content, reordered)).toBe(true);
    expect(sameContent(content, removeBlock(content, 'intro'))).toBe(false);
  });
});
