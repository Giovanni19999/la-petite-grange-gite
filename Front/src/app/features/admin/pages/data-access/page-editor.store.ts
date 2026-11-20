import { HttpErrorResponse } from '@angular/common/http';
import { DestroyRef, Injectable, computed, inject, signal } from '@angular/core';
import { firstValueFrom } from 'rxjs';

import { Lang } from '../../../public/pages/data-access/pages.models';
import { AdminPagesApi } from './admin-pages.api';
import {
  AdminPage,
  ApiErrorBody,
  BlockType,
  DraftBlock,
  PageChanges,
  PageContent,
  PageSettings,
  PageVersion,
} from './admin-pages.models';
import {
  blockIds,
  blockLabel,
  changedBlockIds,
  duplicateBlock,
  insertBlock,
  moveBlock,
  newBlock,
  removeBlock,
  replaceBlock,
  sameContent,
} from './page-content';

/** Delay between the last change and the automatic save of the draft. */
export const AUTOSAVE_DELAY_MS = 1500;

export type SaveStatus = 'saved' | 'pending' | 'saving' | 'invalid' | 'conflict' | 'error';

/** Problems reported by the API when saving, shown next to what they concern. */
export interface SaveProblems {
  /** Slug already used by another page, or invalid slug. */
  slug: string | null;
  /** Every other problem (missing French title, invalid block options…). */
  other: string[];
}

export type PublishOutcome = 'published' | 'changed' | 'failed';

const NO_PROBLEMS: SaveProblems = { slug: null, other: [] };

/**
 * State of the page being edited (F023): draft content, autosave with the draft version it
 * was based on, conflict detection, publication. One instance per editor page.
 */
@Injectable()
export class PageEditorStore {
  private readonly api = inject(AdminPagesApi);

  private readonly pageState = signal<AdminPage | null>(null);
  private readonly contentState = signal<PageContent | null>(null);
  private readonly draftVersion = signal(0);
  private timer: ReturnType<typeof setTimeout> | null = null;
  private saving: Promise<void> | null = null;

  readonly page = this.pageState.asReadonly();
  readonly content = this.contentState.asReadonly();
  readonly lang = signal<Lang>('fr');
  readonly selectedId = signal<string | null>(null);
  readonly status = signal<SaveStatus>('saved');
  readonly problems = signal<SaveProblems>(NO_PROBLEMS);
  /** Names of the blocks the owner changed before a conflict made them reload (to redo). */
  readonly lostChanges = signal<string[]>([]);
  readonly loadError = signal(false);

  readonly selectedBlock = computed(
    () => this.content()?.blocks.find((block) => block.id === this.selectedId()) ?? null,
  );
  /** Something typed is not saved on the server yet. */
  readonly hasUnsavedChanges = computed(() => this.status() !== 'saved');
  /** The draft differs from what visitors see (or the page was never published). */
  readonly hasUnpublishedChanges = computed(() => {
    const content = this.content();
    const published = this.page()?.published ?? null;
    return content !== null && (published === null || !sameContent(content, published));
  });

  constructor() {
    inject(DestroyRef).onDestroy(() => this.clearTimer());
  }

  async load(id: number): Promise<void> {
    this.clearTimer();
    try {
      const page = await firstValueFrom(this.api.get(id));
      this.applyLoaded(page);
      this.loadError.set(false);
    } catch {
      this.loadError.set(true);
    }
  }

  // ----- Editing (each change schedules an automatic save) -----

  addBlock(type: BlockType, index: number): void {
    const content = this.content();
    if (!content) {
      return;
    }
    const block = newBlock(type, blockIds(content));
    this.change(insertBlock(content, index, block));
    this.selectedId.set(block.id);
  }

  updateBlock(block: DraftBlock): void {
    const content = this.content();
    if (content) {
      this.change(replaceBlock(content, block));
    }
  }

  duplicate(id: string): void {
    const content = this.content();
    if (!content) {
      return;
    }
    const next = duplicateBlock(content, id);
    this.change(next);
    const index = next.blocks.findIndex((block) => block.id === id);
    this.selectedId.set(next.blocks[index + 1]?.id ?? null);
  }

  remove(id: string): void {
    const content = this.content();
    if (!content) {
      return;
    }
    this.change(removeBlock(content, id));
    if (this.selectedId() === id) {
      this.selectedId.set(null);
    }
  }

  move(from: number, to: number): void {
    const content = this.content();
    if (content) {
      this.change(moveBlock(content, from, to));
    }
  }

  updateSettings(settings: PageSettings): void {
    const content = this.content();
    if (content) {
      this.change({ ...content, settings });
    }
  }

  private change(content: PageContent): void {
    if (this.status() === 'conflict') {
      return;
    }
    this.contentState.set(content);
    this.status.set('pending');
    this.clearTimer();
    this.timer = setTimeout(() => void this.save(), AUTOSAVE_DELAY_MS);
  }

  // ----- Saving -----

  /** Saves now what is pending (used when leaving the page and before publishing). */
  async flush(): Promise<void> {
    this.clearTimer();
    if (this.saving) {
      await this.saving;
    }
    if (this.status() === 'pending') {
      await this.save();
    }
  }

  /** Tries again after a failed save. */
  retry(): Promise<void> {
    if (this.status() !== 'error') {
      return Promise.resolve();
    }
    this.status.set('pending');
    return this.flush();
  }

  private save(): Promise<void> {
    this.saving ??= this.doSave().finally(() => (this.saving = null));
    return this.saving;
  }

  private async doSave(): Promise<void> {
    const page = this.page();
    const content = this.content();
    if (!page || !content || this.status() === 'conflict') {
      return;
    }
    this.status.set('saving');
    try {
      const result = await firstValueFrom(
        this.api.saveDraft(page.id, this.draftVersion(), content),
      );
      this.draftVersion.set(result.draft_version);
      this.problems.set(NO_PROBLEMS);
      this.status.set('saved');
    } catch (error) {
      this.onSaveError(error);
    }
    // Changed while saving (the answer is about older content): save the newer content too,
    // whether this save succeeded or was refused.
    if (this.content() !== content && this.status() !== 'conflict') {
      this.status.set('pending');
      this.clearTimer();
      this.timer = setTimeout(() => void this.save(), AUTOSAVE_DELAY_MS);
    }
  }

  private onSaveError(error: unknown): void {
    const body = error instanceof HttpErrorResponse ? (error.error as ApiErrorBody | null) : null;
    const code = body?.error?.code;
    if (code === 'version_conflict') {
      this.status.set('conflict');
      return;
    }
    if (code === 'slug_taken') {
      this.problems.set({ slug: body?.error.message ?? '', other: [] });
      this.status.set('invalid');
      return;
    }
    if (code === 'invalid_content') {
      const details = body?.error.details ?? [];
      const slug = details.filter((detail) => detail.startsWith('adresse de page'));
      this.problems.set({
        slug: slug.length > 0 ? slug.join(' ') : null,
        other: details.filter((detail) => !slug.includes(detail)),
      });
      this.status.set('invalid');
      return;
    }
    this.status.set('error');
  }

  /** After a conflict: loads the latest draft and remembers which blocks the owner changed. */
  async reloadAfterConflict(): Promise<void> {
    const page = this.page();
    const mine = this.content();
    if (!page) {
      return;
    }
    await this.load(page.id);
    const latest = this.content();
    if (mine && latest) {
      const changed = new Set(changedBlockIds(latest, mine));
      this.lostChanges.set(
        mine.blocks.filter((block) => changed.has(block.id)).map((block) => blockLabel(block.type)),
      );
    }
  }

  // ----- Publishing and history -----

  /** Saves what is pending, then asks the API for the summary of changes. */
  async changes(): Promise<PageChanges | null> {
    await this.flush();
    const page = this.page();
    if (!page || this.status() !== 'saved') {
      return null;
    }
    return firstValueFrom(this.api.changes(page.id));
  }

  /** Publishes the draft version the owner confirmed. */
  async publish(confirmedVersion: number): Promise<PublishOutcome> {
    const page = this.page();
    if (!page) {
      return 'failed';
    }
    try {
      await firstValueFrom(this.api.publish(page.id, confirmedVersion));
      await this.load(page.id);
      return 'published';
    } catch (error) {
      return error instanceof HttpErrorResponse && error.status === 409 ? 'changed' : 'failed';
    }
  }

  async versions(): Promise<PageVersion[]> {
    const page = this.page();
    return page ? firstValueFrom(this.api.versions(page.id)) : [];
  }

  /** Puts a published version back into the draft (not published). */
  async restore(version: number): Promise<boolean> {
    const page = this.page();
    if (!page) {
      return false;
    }
    await this.flush();
    try {
      await firstValueFrom(this.api.restore(page.id, version));
      await this.load(page.id);
      return true;
    } catch {
      return false;
    }
  }

  private applyLoaded(page: AdminPage): void {
    this.pageState.set(page);
    this.contentState.set(page.draft);
    this.draftVersion.set(page.draft_version);
    this.status.set('saved');
    this.problems.set(NO_PROBLEMS);
    this.lostChanges.set([]);
    if (!page.draft.blocks.some((block) => block.id === this.selectedId())) {
      this.selectedId.set(null);
    }
  }

  private clearTimer(): void {
    if (this.timer !== null) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }
}
