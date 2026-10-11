# Delivery

- Added `/swap/history` with a localized title, hidden Dock, Back-to-Swap fallback,
  login/loading/error/empty states, explicit load-more and retry.
- Swap's history button navigates there. Removed embedded history and history
  fetches from the quote/confirmation page without changing monetary commands.
- Orders expose joined nullable asset symbols/logos even after a pair is disabled.
  Client prefers row metadata, preserves explicit null, falls back for old-server
  omitted fields and tolerates optional pair-catalogue failure.
- Records show both AssetMarks, exact decimal amounts, time, status, rate and fee.
- Shared history pagination now ignores late rejected requests after identity or
  generation change, matching its existing stale-success protection.
- Combined mobile release gate: 844/844, production/test types, PWA/Tauri web
  builds, artifacts and budgets passed. Adapter, route, localization and late
  failure regression cases were added.
- Real isolated MySQL: `convert_routes_list_pairs_and_user_orders` passed,
  including historical metadata after disabling the pair. Rust library 523,
  architecture 11, documentation 1, OpenAPI 14 and strict Clippy passed in the
  combined change. Optional DB branches in library tests can internally skip.
- Ego TaskSpace16: actual guest navigation to the dedicated route, then an
  in-memory authenticated fixture with GET-only mocked HTTP. Twenty rows loaded,
  both configured test images rendered, exact large/tiny amounts preserved,
  load-more requested offset20 and ended at21 rows. 320/390/1280px overflow was
  zero, and clearing the in-memory identity removed every row immediately.
  The fixture used local test images, not claims about production asset artwork.
- TaskSpace16 was closed once after the combined verification.
- No real user login, conversion or funds mutation. Server changes need a
  backend deployment; neither push nor production deployment was performed.
