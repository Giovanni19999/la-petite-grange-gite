/** One line of the summary of changes, e.g. "Bloc ajouté : Texte". */
export interface PublishSummaryLine {
  label: string;
  items: string[];
}

/** What the publish dialog shows: the summary lines and the languages whose texts changed. */
export interface PublishSummary {
  lines: PublishSummaryLine[];
  languages: ('fr' | 'en')[];
}
