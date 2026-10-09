# Configurable home promotion carousel

## Goal

Replace the four selected Pencil home Market Brief blocks and the production mobile equivalent with a compact carousel matching the supplied reference: text at left, artwork at right, pagination below. Administrators configure slides and their internal destinations.

## What I already know

* The active Pencil file is `mobile/pencil/hippo-mobile-uiux.pen`.
* The current selection contains four frames: `r8zcT`, `w2hy2C`, `MnV84`, and `W4ql6`.
* Each selected frame is a duplicated `Market Brief` component-like block.
* The current content is an AI market briefing card with the copy `AI 行情日报`, `三分钟读懂今日市场`, and `暂无平台公告`.
* Two selected variants use the dark filled card treatment and two use the dark bordered card treatment.
* No reusable Pencil components are registered; the selected blocks are independent frames.

## Confirmed Scope (2026-10-08)

* The user supplied a carousel reference and explicitly requested backend configuration and links to activity/news detail pages.
* This expands the initial design-only scope to mobile, admin web, and Rust API/storage.
* Existing news category `promotion` supplies activity content; reuse its editor and published-only detail page instead of duplicating a CMS.
* Support activity detail, news detail, news center, and the existing referral page as typed internal destinations. No arbitrary external URL or JavaScript destination.
* Each slide owns title, description, artwork URL, enabled state, optional locale, and array-order position. At most 12 slides.
* Use a singleton versioned configuration with an atomic audited save. No demo slides are seeded into production.
* The public response includes enabled slides only, excludes unpublished/deleted activity/news targets, and contains no admin metadata.
* Multi-slide carousel supports touch swipe, keyboard navigation, pagination, and automatic rotation. Pause while focused, hovered, document hidden, or reduced motion is requested; clean up listeners and timers.
* Empty configuration hides the carousel; loading/error states do not invent promotion content; errors allow retry.
* Preserve the existing unrelated dirty changes.

## Open Questions

* None blocking for the design slice. The user selected option 1: independent title, description, and image with a separately configured destination.

## Current Delivery Slice

* The latest user instruction is to edit Pencil first and show the visual result before implementation.
* This turn changes only the currently selected Pencil home funding/portfolio areas and task/progress records.
* Backend, admin web, production mobile code, migrations, and real carousel interaction remain deferred until design review.
* Show a referral-themed sample without unconfirmed reward promises. Match the reference with neutral card fill, left-aligned title/description, right artwork, and centered pagination dots with a green active bar.
* Keep the original selected frame IDs and let existing vertical page layouts accommodate the taller carousel.
* The selected portfolio template iteration uses a compact right-side curve beside the balance, with today return values on one line.
* Funding actions use explicit labels `快捷充币` and `链上充币`; the former replaces the previous buy/C2C wording.
* Remove the `1 天` / `7 天` / `30 天` / `180 天` period selector from both portfolio themes.
* Remove the horizontal guide lines behind the portfolio curve in both portfolio themes.
* Keep the dark portfolio theme structurally aligned with the light theme: compact right-side curve and inline today-return values.
* The next design-only iteration targets the selected guest portfolio frames `ZGf2W` / `u1hhY` and funding rows `ysQIB` / `KhrN7`.
* Replace the oversized guest marketing card with a compact unauthenticated asset summary, neutral balance/return placeholders, and explicit login/register actions. Do not invent guest balances or return curves.
* Align guest funding labels and icons with the approved signed-in design: `快捷充币` with a lightning icon and `链上充币` with the deposit icon.
* Hide the selected guest funding-action rows in both unauthenticated themes; guest users should not see these actions.
* Further refine the newly selected guest portfolio frames into a compact prompt strip: no fake balance, no large artwork, login as the primary action, and registration as the secondary action.
* Add restrained image and motion language to the selected guest portfolio frames: themed background image, flowing mint trail, orbit rings, glow, and floating dots. Keep the actual motion deferred to production mobile implementation.
* Preserve the guest header, shortcuts, carousel, markets, bottom navigation, and all signed-in screens.

### Guest Design Checklist

* [x] Inspect the selected guest nodes, signed-in reference, and existing layout warnings.
* [x] Update the light and dark guest portfolio/funding sections.
* [x] Verify both full screens and edited sections for contrast, alignment, clipping, and residual old copy.
* [x] Record the design-only delivery in `docs/superpowers/PROGRESS.md`.
* [x] Refine the selected guest portfolio frames into the compact exchange-style login prompt.
* [x] Add theme-specific image and motion accents without allowing them to cover the login/register content.

## Requirements (evolving)

* Replace only the currently selected Pencil blocks.
* Use the supplied compact neutral card treatment with theme-appropriate colors; allow extra vertical space for indicators.
* Verify the edited Pencil nodes with screenshots and a layout check.
* Add an admin content-operations carousel page with image upload, slide editing, enabled toggles, ordering controls, destination selection, dirty-state protection, and audited save.
* Reuse existing `content.news` read/write permission for this news-linked content operation and register exact route mappings on both ends.
* Reject unsafe image URLs, malformed destinations, invalid IDs, oversized text/lists and stale versions.

## Acceptance Criteria (evolving)

* [ ] The user-approved replacement is present in all four selected locations.
* [ ] No unrelated Pencil nodes are changed.
* [x] The replacement has no clipping, collapse, or overflow in the selected frames.
* [x] Light/dark variants remain visually coherent.
* [x] Portfolio period selectors and curve guide lines are removed from both themes.
* [ ] API validates configuration, persists atomically with audit, and filters public targets correctly.
* [ ] Admin can load, edit, reorder, enable/disable, upload artwork, and save with a reason; conflicts preserve drafts.
* [ ] Mobile displays the live configuration and navigates to the configured existing page.
* [ ] Focused Rust, Web, and Mobile tests and type/build checks pass; visual checks cover desktop admin and narrow/light/dark mobile.
* [ ] The change is recorded in `docs/superpowers/PROGRESS.md`.

## Definition of Done (team quality bar)

* Pencil canvas edit completed and visually checked.
* Relevant design/task records updated.
* No unrelated code or design changes included.

## Out of Scope (explicit)

* No changes to unselected Pencil screens.
* No PC-client redesign, external links, reward/financial rules, automatic publishing of draft news, scheduled publication, or separate activity CMS.
* No production database writes, deployment, git commit, or push.

## Technical Notes

* Pencil app state and selected node structure were inspected through `mcp__pencil`.
* Mobile design guidelines are indexed in `.trellis/spec/mobile/index.md`.
* Production home currently uses a live market breadth block in `HomeView.vue`; replace only that block, retaining portfolio and market data.
* Plan: design -> persisted/configurable API -> admin editor -> mobile carousel -> regression and visual validation -> progress record.
