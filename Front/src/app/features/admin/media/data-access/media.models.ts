/** A text in French and English (the English one falls back to French). */
export interface MediaText {
  fr: string;
  en: string;
}

/** One WebP version of an image. */
export interface MediaSource {
  width: number;
  url: string;
}

export interface FocalPoint {
  x: number;
  y: number;
}

/** A media of the library, as the admin API returns it (F003). */
export interface Media {
  id: number;
  width: number;
  height: number;
  /** WebP versions, ascending widths. */
  sources: MediaSource[];
  alt: MediaText;
  caption: MediaText;
  focal: FocalPoint;
  missing_alt: boolean;
  used_in: { page_id: number; title: string }[];
  created_at: string;
}

export interface MediaDetails {
  alt: MediaText;
  caption: MediaText;
  focal: FocalPoint;
}

export interface UploadResult {
  uploaded: Media[];
  refused: { file_name: string; reason: string }[];
}
