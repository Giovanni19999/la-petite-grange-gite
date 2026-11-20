import { Injectable, signal } from '@angular/core';

import { Lang } from './pages.models';

/** URLs of the page being displayed in each language, for the language switch. */
@Injectable({ providedIn: 'root' })
export class CurrentPage {
  readonly alternates = signal<Record<Lang, string> | null>(null);
}
