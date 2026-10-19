# New coin presentation and logo propagation

## Goal
Apply the requested Mobile new-coin copy cleanup and support uploading a coin logo that stays consistent when the project is listed on spot.

## Requirements
- Hide configuration timezone/local-time notice, automatic/manual advancement badges, distribution warning, and realtime-subscription explanatory sentence on public new-coin pages.
- Rename distribution to 派发, listing to 上市, and exit fee to 手续费 (with equivalent localized labels).
- Reuse the existing image storage and asset-logo contracts. Support logo input on new-coin creation and preserve it through approval and pair publication/listing.
- Preserve policy timestamps, workflow behavior, authorization, existing applications, and idempotent retries.

## Technical notes
Exact requested strings belong to rust-chain, not the initial hipoex-nacos workspace. The user confirmed the public Mobile client; PC copy is out of scope. Public new coins read assets.logo_url; market pairs expose both pair and asset logo metadata. Previously agent approval discarded the Logo and pair publication omitted it; both paths now preserve the canonical asset image.

## Acceptance criteria
- [x] Requested public text is removed/replaced on Mobile without hiding times or changing policy.
- [x] Uploaded logo is validated and persisted, previewable during creation/review.
- [x] Listing exposes the same logo on the spot pair, including existing-asset projects.
- [x] Targeted tests, type checks, applicable build checks, and progress record complete.

## Out of scope
No financial behavior changes, production uploads, deployment, pushes, or changes to the Java repository for this request.

## Validation outcome
Mobile release gate passed (854 tests). Web suite passed (118 files, 1130 tests), with typecheck, lint, production policy, coverage and build passing. Web bundle budget remains above its existing limit: 2,715,299 vs 2,600,000 bytes (previous recorded baseline 2,712,495); limits are unchanged. Eleven isolated MySQL/Redis route cases, backend architecture/documentation/OpenAPI checks and strict Clippy passed. Browser fixtures verified Mobile copy/Logo at 320/390/448px in light/dark and Admin upload preview at 1280px.

Deployment requires migration 0155 before running the new backend. No production DB, upload storage or deployment was changed.
