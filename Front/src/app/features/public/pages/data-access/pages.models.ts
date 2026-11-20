/** Language of the public site (ADR-0013). */
export type Lang = 'fr' | 'en';

export type Background = 'background' | 'muted' | 'primary';
export type Spacing = 'small' | 'medium' | 'large';
export type Align = 'left' | 'center' | 'right';

/** An image of the media library, resolved by the API for the page language (F003). */
export interface PublicImage {
  id: number;
  width: number;
  height: number;
  /** WebP versions, ascending widths. */
  sources: { width: number; url: string }[];
  alt: string;
  caption: string;
  focal_x: number;
  focal_y: number;
}

interface BlockBase {
  id: string;
  background: Background;
  spacing: Spacing;
  hide_on_mobile: boolean;
  /** Editor preview only (F023): texts shown in French because the English one is empty. */
  fallback?: string[];
}

export interface HeroBlock extends BlockBase {
  type: 'hero';
  image: PublicImage | null;
  title: string;
  subtitle: string;
  button_label: string;
  button_link: string | null;
  align: Align;
  height: 'small' | 'medium' | 'large';
  darken: number;
}

export interface TextBlock extends BlockBase {
  type: 'text';
  body: string;
  width: 'narrow' | 'wide';
  align: Align;
}

export interface TextImageBlock extends BlockBase {
  type: 'text_image';
  body: string;
  image: PublicImage | null;
  image_position: 'left' | 'right';
  ratio: '50-50' | '33-67';
}

export interface ColumnsBlock extends BlockBase {
  type: 'columns';
  items: { title: string; body: string }[];
}

export interface GalleryBlock extends BlockBase {
  type: 'gallery';
  images: PublicImage[];
  columns: number;
}

/** A block with its texts already resolved in the page language by the API. */
export type PublicBlock = HeroBlock | TextBlock | TextImageBlock | ColumnsBlock | GalleryBlock;

export interface PublicPage {
  lang: Lang;
  title: string;
  meta_description: string;
  alternates: Record<Lang, string>;
  blocks: PublicBlock[];
}

export interface MenuItem {
  title: string;
  path: string;
}
