import { Align, Background, Lang, Spacing } from '../../../public/pages/data-access/pages.models';

/** A text in French and English (F002). English falls back to French when empty. */
export interface Localized {
  fr: string;
  en: string;
}

interface DraftBlockBase {
  id: string;
  visible: boolean;
  background: Background;
  spacing: Spacing;
  hide_on_mobile: boolean;
}

export interface DraftHeroBlock extends DraftBlockBase {
  type: 'hero';
  image: number | null;
  title: Localized;
  subtitle: Localized;
  button_label: Localized;
  button_link: string | null;
  align: Align;
  height: 'small' | 'medium' | 'large';
  darken: number;
  /** Fixed photo (`image`) or carousel (`images`), F024. */
  mode: 'image' | 'carousel';
  images: number[];
  autoplay: boolean;
  interval: number;
}

export interface DraftTextBlock extends DraftBlockBase {
  type: 'text';
  body: Localized;
  width: 'narrow' | 'wide';
  align: Align;
}

export interface DraftTextImageBlock extends DraftBlockBase {
  type: 'text_image';
  body: Localized;
  image: number | null;
  image_position: 'left' | 'right';
  ratio: '50-50' | '33-67';
}

export interface DraftColumnItem {
  title: Localized;
  body: Localized;
}

export interface DraftColumnsBlock extends DraftBlockBase {
  type: 'columns';
  items: DraftColumnItem[];
  align: Align;
}

export interface DraftGalleryBlock extends DraftBlockBase {
  type: 'gallery';
  images: number[];
  columns: number;
}

export interface DraftImageBlock extends DraftBlockBase {
  type: 'image';
  image: number | null;
  width: 'narrow' | 'wide' | 'full';
}

export interface DraftCarouselBlock extends DraftBlockBase {
  type: 'carousel';
  images: number[];
  height: 'small' | 'medium' | 'large';
  autoplay: boolean;
  interval: number;
}

export interface DraftAvailabilityBlock extends DraftBlockBase {
  type: 'availability';
  months: number;
}

/** A block as stored in the draft: texts in both languages. */
export type DraftBlock =
  | DraftHeroBlock
  | DraftTextBlock
  | DraftTextImageBlock
  | DraftColumnsBlock
  | DraftGalleryBlock
  | DraftImageBlock
  | DraftCarouselBlock
  | DraftAvailabilityBlock;

export type BlockType = DraftBlock['type'];

export interface PageSettings {
  slug: Localized;
  title: Localized;
  meta_description: Localized;
  in_menu: boolean;
  menu_order: number;
}

/** What is saved as draft, published and versioned. */
export interface PageContent {
  settings: PageSettings;
  blocks: DraftBlock[];
}

/** Line of the pages list. */
export interface AdminPageSummary {
  id: number;
  is_home: boolean;
  title: string;
  slug: string;
  draft_version: number;
  published_at: string | null;
  unpublished_changes: boolean;
}

export interface AdminPage {
  id: number;
  is_home: boolean;
  draft_version: number;
  draft: PageContent;
  published: PageContent | null;
  published_at: string | null;
}

export interface ChangeSummary {
  settings_changed: boolean;
  blocks_added: string[];
  blocks_removed: string[];
  blocks_changed: string[];
  blocks_moved: boolean;
  languages: Lang[];
}

export interface PageChanges {
  draft_version: number;
  changes: ChangeSummary;
  nothing_to_publish: boolean;
}

export interface PageVersion {
  number: number;
  published_at: string;
  by_owner: boolean;
}

/** Error body of the API: `{"error": {"code", "message", "details"?}}`. */
export interface ApiErrorBody {
  error: { code: string; message: string; details?: string[] };
}
