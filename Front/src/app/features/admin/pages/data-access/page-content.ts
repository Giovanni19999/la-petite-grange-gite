import { Lang, PublicBlock } from '../../../public/pages/data-access/pages.models';
import { PublishSummary, PublishSummaryLine } from '../../../../shared/publish/publish-summary';
import { toPublicImage } from '../../media/data-access/media.api';
import { Media } from '../../media/data-access/media.models';
import {
  BlockType,
  ChangeSummary,
  DraftBlock,
  DraftColumnItem,
  Localized,
  PageContent,
} from './admin-pages.models';

/** Pure functions on draft page content (F023): no state, no HTTP. */

/** Name of each block type in the admin, in the order offered by the block picker. */
export const BLOCK_TYPES: readonly { type: BlockType; label: string; description: string }[] = [
  { type: 'hero', label: 'Bandeau', description: 'Grande image, titre, sous-titre et bouton' },
  { type: 'text', label: 'Texte', description: 'Paragraphes, titres, listes et liens' },
  { type: 'text_image', label: 'Texte et image', description: 'Texte à côté d’une image' },
  { type: 'columns', label: 'Colonnes', description: '2 à 4 colonnes avec titre et texte' },
  { type: 'gallery', label: 'Galerie', description: 'Grille de photos' },
  { type: 'image', label: 'Image', description: 'Une photo seule avec sa légende' },
  { type: 'carousel', label: 'Carrousel', description: 'Photos qui défilent' },
  {
    type: 'availability',
    label: 'Disponibilités',
    description: 'Calendrier, prix par nuit et devis',
  },
];

/** Seconds between two photos of a carousel, by default (F024). */
export const DEFAULT_CAROUSEL_INTERVAL = 6;

export function blockLabel(type: BlockType): string {
  return BLOCK_TYPES.find((entry) => entry.type === type)?.label ?? type;
}

export const emptyText = (): Localized => ({ fr: '', en: '' });

const emptyColumn = (): DraftColumnItem => ({ title: emptyText(), body: emptyText() });

/** Maximum length of a block id accepted by the API. */
const BLOCK_ID_MAX_LENGTH = 40;

/** A block id unique in the page: `<type>-<random>` (letters, digits, `-`, `_`). */
export function newBlockId(type: BlockType, taken: ReadonlySet<string>): string {
  for (;;) {
    const id = `${type}-${Math.random().toString(36).slice(2, 8)}`.slice(0, BLOCK_ID_MAX_LENGTH);
    if (!taken.has(id)) {
      return id;
    }
  }
}

/** A new block with empty texts and the default settings of the API. */
export function newBlock(type: BlockType, taken: ReadonlySet<string>): DraftBlock {
  const base = {
    id: newBlockId(type, taken),
    visible: true,
    background: 'background' as const,
    spacing: 'medium' as const,
    hide_on_mobile: false,
  };
  switch (type) {
    case 'hero':
      return {
        ...base,
        type,
        image: null,
        title: emptyText(),
        subtitle: emptyText(),
        button_label: emptyText(),
        button_link: null,
        align: 'center',
        height: 'medium',
        darken: 0,
        mode: 'image',
        images: [],
        autoplay: true,
        interval: DEFAULT_CAROUSEL_INTERVAL,
      };
    case 'text':
      return { ...base, type, body: emptyText(), width: 'narrow', align: 'left' };
    case 'text_image':
      return {
        ...base,
        type,
        body: emptyText(),
        image: null,
        image_position: 'right',
        ratio: '50-50',
      };
    case 'columns':
      return {
        ...base,
        type,
        items: [emptyColumn(), emptyColumn(), emptyColumn()],
        align: 'left',
      };
    case 'gallery':
      return { ...base, type, images: [], columns: 3 };
    case 'image':
      return { ...base, type, image: null, width: 'wide' };
    case 'carousel':
      return {
        ...base,
        type,
        images: [],
        height: 'medium',
        autoplay: false,
        interval: DEFAULT_CAROUSEL_INTERVAL,
      };
    case 'availability':
      return { ...base, type, months: 2 };
  }
}

export function blockIds(content: PageContent): Set<string> {
  return new Set(content.blocks.map((block) => block.id));
}

/** Inserts `block` at `index` (clamped). */
export function insertBlock(content: PageContent, index: number, block: DraftBlock): PageContent {
  const blocks = [...content.blocks];
  blocks.splice(Math.max(0, Math.min(index, blocks.length)), 0, block);
  return { ...content, blocks };
}

/** Copy of the block right after it, with a new id. */
export function duplicateBlock(content: PageContent, id: string): PageContent {
  const index = content.blocks.findIndex((block) => block.id === id);
  if (index < 0) {
    return content;
  }
  const original = content.blocks[index];
  const copy = { ...structuredClone(original), id: newBlockId(original.type, blockIds(content)) };
  return insertBlock(content, index + 1, copy);
}

export function removeBlock(content: PageContent, id: string): PageContent {
  return { ...content, blocks: content.blocks.filter((block) => block.id !== id) };
}

/** Moves the block at `from` to `to` (both clamped). */
export function moveBlock(content: PageContent, from: number, to: number): PageContent {
  const last = content.blocks.length - 1;
  const source = Math.max(0, Math.min(from, last));
  const target = Math.max(0, Math.min(to, last));
  if (source === target) {
    return content;
  }
  const blocks = [...content.blocks];
  const [moved] = blocks.splice(source, 1);
  blocks.splice(target, 0, moved);
  return { ...content, blocks };
}

export function replaceBlock(content: PageContent, block: DraftBlock): PageContent {
  return {
    ...content,
    blocks: content.blocks.map((current) => (current.id === block.id ? block : current)),
  };
}

/** Whether the text of `lang` is missing and the French one is shown instead. */
export function isFallback(text: Localized, lang: Lang): boolean {
  return lang === 'en' && text.en.trim() === '' && text.fr.trim() !== '';
}

/** Text shown in `lang`: English falls back to French, as on the public site. */
export function resolveText(text: Localized, lang: Lang): string {
  return lang === 'en' && text.en.trim() !== '' ? text.en : text.fr;
}

/**
 * The block as the public site receives it in `lang` (texts resolved), plus the names of the
 * texts that fall back to French, shown greyed out in the editor preview.
 */
export function toPreviewBlock(
  block: DraftBlock,
  lang: Lang,
  media: ReadonlyMap<number, Media> = new Map(),
): PublicBlock {
  const fallback: string[] = [];
  const image = (id: number | null) => {
    const found = id === null ? undefined : media.get(id);
    return found ? toPublicImage(found, lang) : null;
  };
  const text = (value: Localized, name: string): string => {
    if (isFallback(value, lang)) {
      fallback.push(name);
    }
    return resolveText(value, lang);
  };
  const common = {
    id: block.id,
    background: block.background,
    spacing: block.spacing,
    // The editor still shows it on a phone, greyed out with a badge (F023-S09).
    hide_on_mobile: false,
  };
  let preview: PublicBlock;
  switch (block.type) {
    case 'hero':
      preview = {
        ...common,
        type: 'hero',
        image: image(block.image),
        title: text(block.title, 'title'),
        subtitle: text(block.subtitle, 'subtitle'),
        button_label: text(block.button_label, 'button_label'),
        button_link: block.button_link,
        align: block.align,
        height: block.height,
        darken: block.darken,
        mode: block.mode,
        images: block.images.flatMap((id) => image(id) ?? []),
        // No scrolling in the editor: the owner looks at one photo at a time.
        autoplay: false,
        interval: block.interval,
      };
      break;
    case 'text':
      preview = {
        ...common,
        type: 'text',
        body: text(block.body, 'body'),
        width: block.width,
        align: block.align,
      };
      break;
    case 'text_image':
      preview = {
        ...common,
        type: 'text_image',
        body: text(block.body, 'body'),
        image: image(block.image),
        image_position: block.image_position,
        ratio: block.ratio,
      };
      break;
    case 'columns':
      preview = {
        ...common,
        type: 'columns',
        items: block.items.map((item, index) => ({
          title: text(item.title, `items.${index}.title`),
          body: text(item.body, `items.${index}.body`),
        })),
        align: block.align,
      };
      break;
    case 'gallery':
      preview = {
        ...common,
        type: 'gallery',
        images: block.images.flatMap((id) => image(id) ?? []),
        columns: block.columns,
      };
      break;
    case 'image':
      preview = { ...common, type: 'image', image: image(block.image), width: block.width };
      break;
    case 'carousel':
      preview = {
        ...common,
        type: 'carousel',
        images: block.images.flatMap((id) => image(id) ?? []),
        height: block.height,
        autoplay: false,
        interval: block.interval,
      };
      break;
    case 'availability':
      preview = { ...common, type: 'availability', months: block.months };
      break;
  }
  return fallback.length > 0 ? { ...preview, fallback } : preview;
}

/** Lower-case letters, digits and single hyphens, as the API requires for slugs. */
export function slugify(title: string): string {
  return title
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

/** Content of a new page: French title and slug; English slug starts as the French one. */
export function newPageContent(title: string, slug: string, menuOrder: number): PageContent {
  return {
    settings: {
      slug: { fr: slug, en: slug },
      title: { fr: title, en: '' },
      meta_description: emptyText(),
      in_menu: false,
      menu_order: menuOrder,
    },
    blocks: [],
  };
}

/** Ids of the blocks that differ between two versions of the content (added or changed). */
export function changedBlockIds(base: PageContent, mine: PageContent): string[] {
  const before = new Map(base.blocks.map((block) => [block.id, JSON.stringify(block)]));
  return mine.blocks
    .filter((block) => before.get(block.id) !== JSON.stringify(block))
    .map((block) => block.id);
}

/** Summary of changes as shown in the publish dialog, with block names instead of ids. */
export function publishSummary(
  changes: ChangeSummary,
  draft: PageContent,
  published: PageContent | null,
): PublishSummary {
  const typeOf = new Map<string, BlockType>(
    [...(published?.blocks ?? []), ...draft.blocks].map((block) => [block.id, block.type]),
  );
  const names = (ids: string[]) =>
    ids.map((id) => {
      const type = typeOf.get(id);
      return type ? blockLabel(type) : id;
    });
  const lines: PublishSummaryLine[] = [];
  if (changes.settings_changed) {
    lines.push({ label: 'Réglages de la page modifiés', items: [] });
  }
  const groups: [string[], string, string][] = [
    [changes.blocks_added, 'Bloc ajouté', 'Blocs ajoutés'],
    [changes.blocks_changed, 'Bloc modifié', 'Blocs modifiés'],
    [changes.blocks_removed, 'Bloc supprimé', 'Blocs supprimés'],
  ];
  for (const [ids, one, many] of groups) {
    if (ids.length > 0) {
      lines.push({
        label: ids.length === 1 ? one : `${ids.length} ${many.toLowerCase()}`,
        items: names(ids),
      });
    }
  }
  if (changes.blocks_moved) {
    lines.push({ label: 'Ordre des blocs modifié', items: [] });
  }
  return { lines, languages: changes.languages };
}

/** Deep equality of two contents, whatever the order of the object keys. */
export function sameContent(a: PageContent, b: PageContent): boolean {
  return stableJson(a) === stableJson(b);
}

function stableJson(value: unknown): string {
  return JSON.stringify(value, (_key, current: unknown) =>
    current !== null && typeof current === 'object' && !Array.isArray(current)
      ? Object.fromEntries(
          Object.entries(current as Record<string, unknown>).sort(([a], [b]) => a.localeCompare(b)),
        )
      : current,
  );
}
