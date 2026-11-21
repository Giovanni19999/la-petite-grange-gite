---
id: F023
title: "Visual page editor in the admin (preview, drag and drop, FR/EN, publish dialog)"
status: implemented
version: 2
adr: [ADR-0009, ADR-0015, ADR-0004, ADR-0013]
---

# F023 - Visual page editor in the admin (preview, drag and drop, FR/EN, publish dialog)

## User story

As an owner,
I want to change the pages of the site in an editor that shows them as visitors will see them,
so that I can update texts, images and layout myself, on a computer or on my phone, without breaking the site.

## Business rules

* The editor uses the page API of F002 only (no new endpoint): list, create, delete, get, save draft, summary of changes, publish, versions, restore.
* **Pages list** (`/admin/pages`): every page with its French title, slug, a "home" marker, its last publication date and a "not published" marker (draft different from the published page, or never published). The owner creates a page (French title and slug required) and deletes a page after confirmation; the home page cannot be deleted.
* **Editor** (`/admin/pages/:id`): a live preview of the draft drawn with the **same block components as the public site**, and a panel to edit the selected block (side panel on wide screens, bottom drawer on phones).
* **Blocks**: add (a "+" between blocks opens an illustrated choice of the F002 block types), select, edit, duplicate, delete (after confirmation), move with a drag handle and with up / down buttons (keyboard accessible). A new block starts with sensible empty content and default settings.
* **Block panel**: the block's texts in the current language and its settings (F002 options and common settings: visible, background, spacing, hide on mobile). Rich text uses the Tiptap editor limited to the formats the API keeps (ADR-0015). Image fields show a placeholder until the media library (F003).
* **Languages**: a FR / EN switch changes the language of the texts being edited and of the preview; layout and settings are shared by both languages. An empty English text shows the French one greyed out in the preview (the public fallback) and is marked "not translated" in the panel.
* **Page settings** (dialog): slug, title and meta description per language, shown in the menu, menu position.
* **Device preview**: computer / tablet / phone widths; "hide on mobile" blocks stay shown in the phone preview and on a phone, greyed out with the badge "Masqué sur téléphone", so that the owner can still select and edit them; only the public site hides them.
* **Saving**: the draft is saved automatically a short delay after the last change (and immediately when leaving the page), sending the draft version it was based on. A status shows "Saving…", "Saved" or the error. Validation errors from the API are shown next to the block or setting concerned and nothing is lost in the editor.
* **Conflict**: if the draft was changed elsewhere (another tab, the other owner: HTTP 409), autosave stops and the owner is told; they reload the latest draft (their unsaved change is shown so they can redo it).
* **Publish**: the "Publish" button opens a shared confirmation dialog (`ConfirmPublishDialog`, spartan/ui Alert Dialog, keyboard accessible, focus on "Cancel" by default) showing the summary of changes from the API (blocks added, removed, changed, moved; settings changed; languages concerned) with **Publish** and **Cancel**. Publishing sends the draft version that was summarised; on HTTP 409 the dialog shows the new summary to confirm again. When there is nothing to publish, the button is disabled.
* **History**: the list of published versions (number and publication date) with "Restore"; restoring (after confirmation) loads that content into the draft, which stays unpublished until the owner publishes.
* Leaving the editor while a save is pending or failed asks for confirmation.
* All admin texts are in French; every action is usable with the keyboard and on a 360 px wide screen.

## Acceptance criteria

```gherkin
Feature: Visual page editor

  Background:
    Given the owner is logged in
    And the default pages exist and are published

  Scenario: F023-S01 Pages list
    When the owner opens "/admin/pages"
    Then every page is listed with its French title, slug and home marker
    And pages whose draft differs from the published page are marked "not published"

  Scenario: F023-S02 Create and delete a page
    When the owner creates a page with the French title "Activités" and the slug "activites"
    Then the editor opens on the new empty page
    When the owner deletes that page and confirms
    Then it is no longer listed
    And the home page offers no delete action

  Scenario: F023-S03 Preview uses the public rendering
    When the owner opens the home page in the editor
    Then the preview shows the draft blocks with the same components as the public site

  Scenario: F023-S04 Edit a block and autosave
    When the owner selects the hero block and changes its French title
    Then the preview shows the new title at once
    And the draft is saved automatically with the version it was based on
    And the status shows "Saved"
    And visitors still see the published title

  Scenario: F023-S05 Add, duplicate, move and delete blocks
    When the owner adds a "text" block after the hero block
    And duplicates it, moves the copy up with the keyboard, then deletes it and confirms
    Then the draft contains the hero block followed by one text block

  Scenario: F023-S06 Reorder with drag and drop
    When the owner drags the third block above the first one
    Then the preview and the saved draft show the new order

  Scenario: F023-S07 French and English
    Given the English title of the hero block is empty
    When the owner switches to EN
    Then the preview shows the French title greyed out and the panel marks it "not translated"
    When the owner types an English title
    Then the French title is unchanged

  Scenario: F023-S08 Rich text limited to allowed formats
    When the owner pastes text containing a table, an image and a script into a text block
    Then only the allowed formats are kept in the editor
    And the toolbar only offers bold, italic, lists, headings 2 to 4 and links

  Scenario: F023-S09 Device preview
    Given a block is set to "hide on mobile"
    When the owner chooses the phone preview, or edits the page on a phone
    Then that block is still shown, greyed out, with the badge "Masqué sur téléphone"
    And the owner can select and edit it
    And visitors on a phone do not see it

  Scenario: F023-S10 Validation error from the API
    When the owner sets a slug already used by another page
    Then the error is shown next to the slug field
    And the owner's other changes are kept

  Scenario: F023-S11 Draft changed elsewhere
    Given the draft was saved from another tab after the owner opened it
    When the owner changes a text
    Then autosave stops and the owner is told the page changed elsewhere
    And the owner can reload the latest draft

  Scenario: F023-S12 Publish with confirmation
    Given the owner changed a text and added a block
    When the owner clicks "Publish"
    Then a dialog lists one block added and one block changed, with the languages concerned
    And the focus is on "Cancel"
    When the owner confirms
    Then visitors see the new content and the page is no longer marked "not published"

  Scenario: F023-S13 Publish refused because the draft changed
    Given the publish dialog is open
    And the draft is saved again from another tab
    When the owner confirms
    Then nothing is published
    And the dialog shows the new summary to confirm again

  Scenario: F023-S14 Restore a version
    When the owner restores version 1 from the history and confirms
    Then the editor shows that version's content as the draft
    And visitors still see the current published page

  Scenario: F023-S15 Mobile and keyboard
    Given a 360 px wide screen
    When the owner edits a block
    Then the block panel opens as a bottom drawer
    And every action of the editor can be done with the keyboard
```

## Out of scope

* Media library, image upload, crop and focal point (F003): image fields are placeholders here.
* Reordering the menu by drag and drop across pages (menu position is a page setting here), data blocks (prices, availability, reviews, contact, booking), themes (F020), automatic translation, editing two pages at once, real-time co-editing.

## Tests

| Scenario | Test |
|---|---|
| F023-S01 | `Front/src/app/features/admin/pages/pages-list/pages-list.page.spec.ts` |
| F023-S02 | `Front/src/app/features/admin/pages/pages-list/pages-list.page.spec.ts`, `Front/src/app/features/admin/pages/data-access/page-content.spec.ts` |
| F023-S03 | `Front/src/app/features/admin/pages/editor/page-editor.page.spec.ts` |
| F023-S04 | `Front/src/app/features/admin/pages/data-access/page-editor.store.spec.ts` |
| F023-S05 | `Front/src/app/features/admin/pages/data-access/page-content.spec.ts`, `Front/src/app/features/admin/pages/editor/page-editor.page.spec.ts` |
| F023-S06 | `Front/src/app/features/admin/pages/data-access/page-content.spec.ts` (new order); the drag gesture itself is checked by hand in the browser |
| F023-S07 | `Front/src/app/features/admin/pages/data-access/page-content.spec.ts`, `Front/src/app/features/admin/pages/data-access/page-editor.store.spec.ts`, `Front/src/app/features/admin/pages/editor/page-editor.page.spec.ts` |
| F023-S08 | `Front/src/app/shared/rich-text/rich-text-editor.spec.ts` |
| F023-S09 | `Front/src/app/features/admin/pages/editor/page-editor.page.spec.ts` |
| F023-S10 | `Front/src/app/features/admin/pages/data-access/page-editor.store.spec.ts` |
| F023-S11 | `Front/src/app/features/admin/pages/data-access/page-editor.store.spec.ts` |
| F023-S12 | `Front/src/app/features/admin/pages/data-access/page-content.spec.ts`, `Front/src/app/features/admin/pages/data-access/page-editor.store.spec.ts`, `Front/src/app/features/admin/pages/editor/page-editor.page.spec.ts` |
| F023-S13 | `Front/src/app/features/admin/pages/data-access/page-editor.store.spec.ts` |
| F023-S14 | `Front/src/app/features/admin/pages/data-access/page-editor.store.spec.ts` |
| F023-S15 | `Front/src/app/features/admin/pages/editor/page-editor.page.spec.ts` |

## Changelog

* v1 (2026-11-20): first version.
* v2 (2026-11-21): "hide on mobile" blocks stay shown (greyed out, badge) in the phone preview and when editing on a phone, at the owners' request; they could not be edited from a phone.
