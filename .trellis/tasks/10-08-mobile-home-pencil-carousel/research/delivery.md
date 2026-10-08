# Home Carousel Delivery Evidence

Date: 2026-10-08, Asia/Hong_Kong.

## Implementation

- Rust owns public filtering, a versioned singleton document, independent
  content read/write permissions, transactionally coupled audit, and OpenAPI.
- Migration `0146_home_carousel.sql` starts with an empty document, not an
  example campaign. Existing migrations were not modified.
- Admin `/admin/home/carousel` supports add/remove, order, enablement, locale,
  text, safe image URL/shared upload, typed destination, reason, differences,
  read-only mode, and conflict-preserved drafts.
- Mobile uses the public API with no Authorization, localized requests,
  generation isolation, typed navigation, directional gestures, 5500ms autoplay,
  focus/hover/touch/hidden/reduced-motion pause, and disposal. The follow-up below
  removes the earlier explicit play/pause control to match the selected design.
- Selected guest/member structures and themes are implemented with tracked
  assets. Private wallet/return requests remain authenticated. Real return data
  and lossless amounts are retained; no sample financial values enter production.
- Compact pagination uses a single native range over the decorative dots, so
  five dots occupy 62px instead of five 44px buttons. A 12-page rail is 139px.

## Initial Verification

| Check | Result |
| --- | --- |
| `cargo test --test openapi_routes --test backend_documentation --test backend_architecture --lib` with isolated DB variable | 511 library, 14 route/OpenAPI, 1 documentation, 11 architecture passed |
| `cargo check --all-targets --all-features` | Passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `npm --prefix mobile run release:gate` after compact pagination change | 799 tests, production/test typing, PWA/Tauri builds, artifacts, bundles, source/test budgets passed |
| `npm --prefix web test` | 106 files, 999 tests passed |
| Web `typecheck`, `lint`, `build` | Passed; fixture added to TypeScript include and all three rerun after final fixture changes |
| Web `test:production-policy` | 15 passed |
| Web `test:coverage` | 23 passed; statements 85.61%, branches 81.78%, functions 85.33%, lines 92.87% |
| Web `budget` | Failed: total JS raw 2626539 bytes versus 2600000 |
| `task.py validate` and `git diff --check` | Passed |

Logs are local session artifacts in `/tmp/home-rust-tests.log`,
`/tmp/home-rust-check.log`, `/tmp/home-rust-clippy.log`,
`/tmp/home-mobile-gate-final.log`, and `/tmp/home-web-{tests,build,policy,coverage}.log`.
The Admin budget was already failing in earlier work; this feature adds bytes
and does not solve the existing release blocker. No budget threshold changed.

## Database Evidence

The route test used a dedicated `127.0.0.1:13316/home_carousel_test` database
under `/tmp/home-carousel-mysql.LIQzJn`. It actually exercised all migrations
twice, singleton enforcement, anonymous/read-only denial, revision conflicts,
language and publication filtering, unpublished news removal, clearing,
same-revision concurrent requests (one 200, one 409), matching audit snapshots,
and configuration rollback when audit insertion fails.
The test intentionally skips without `HOME_CAROUSEL_TEST_DATABASE_URL`; the
recorded run supplied that variable. No production database was accessed.

## Initial Browser Evidence

Ego TaskSpace 10 was reused throughout. Fixtures render the actual Mobile App
and Admin editor, replacing only network/socket responses:

- Mobile: `http://localhost:5182/tests/fixtures/home-carousel-browser.html`
  supports `theme`, `member`, `locale=en`, `long`, `slides`, and `error`.
- Admin: `http://localhost:5184/tests/fixtures/home-carousel.html`.

Final 16 combinations of 320/360/390/448px, light/dark, guest/member had no
document horizontal overflow. Guest card measured 128px, standard member
section 129px, carousel 108px, artwork 76px. All images loaded, guests made
no wallet requests, and public carousel requests carried no Authorization.
English 320px and English long-amount 448px also had no horizontal overflow;
long amounts retained every digit. English guest copy can increase height.

Browser interactions confirmed automatic page advance, explicit pause lasting
longer than 5500ms after blur, native range End/ArrowLeft selection up to page
12, reduced-motion pause, image failure placeholder, and balance/return privacy.
Earlier checks covered empty/single/error states. Directional swipes, vertical
scroll exclusion, hidden state and stale lifecycle behavior also have direct
production-handler tests.

Admin browser checks covered desktop 1440px and 390px, add/reorder, reason
required before confirmation, successful save from revision 1 to 2, upload
request count exactly one with image URL populated, and no document horizontal overflow.
Read-only and 409-preserved draft behavior are covered by real Semi UI tests.

Final viewed screenshots:

- `/tmp/home-settled-390-light-guest.png`
- `/tmp/home-settled-390-light-member.png`
- `/tmp/home-settled-390-dark-guest.png`
- `/tmp/home-settled-390-dark-member.png`
- `/tmp/home-final-320-en-guest.png`
- `/tmp/home-final-448-en-long.png`
- `/tmp/home-admin-1440.png`
- `/tmp/home-admin-390.png`

Earlier `home-final-390-*` screenshots were captured during route transitions;
the `home-settled-*` set waits for opacity and animation completion.

## Boundaries and Handoff

- These are isolated browser fixtures and MySQL integration tests, not
  production end-to-end, real-device, or production object-storage verification.
- The follow-up below supersedes the initial 44px guest controls, 44px pagination
  rail, and explicit pause difference. Content, real history geometry and
  narrow-screen wrapping still preclude a claim of exact static pixel identity.
- A real two-point history produces a straight segment. Do not imitate the
  illustrated Pencil curve by inventing data.
- The sample fixture images are not production defaults. Deploy migration 0146,
  configure real content, and grant `system.uploads.write` separately when needed.
- Resolve the Admin bundle blocker before a full release. No deployment or push
  was performed. Preserve all unrelated theme, chart, Pencil and lockfile changes.
- Commit requires user confirmation per workflow phase 3.4; task stays active
  rather than archiving an incompletely approved visual/release gate.
- The dedicated MySQL instance was shut down through its private socket.
  Ego TaskSpace 10 was finished once with no retained pages. Development
  servers on 5182 (Mobile) and 5184 (Admin) remain available for preview.

## Pencil Refinement Follow-Up

Compared the current Pencil nodes `ZGf2W`, `u1hhY`, `ODxuP` and `GBzuP`
again, including both themes and authentication states.

- Guest card: 128px normal height, 18x16px inner padding, inset border and
  theme-specific shadow, 4px copy gaps, 78x30px action faces with 6px spacing,
  transparent secondary action in light mode, and non-interactive outline trails.
- Member card: 128px normal height, inherited heading/return margins reset,
  14px visible eye icon with an extended hit area, 86px chart region and an
  8x8px endpoint positioned from the unchanged real-data geometry.
- Browser computed-style inspection caught the prototype's `.positive
  !important` overriding the intended mint chart. Plot tones now use
  `data-tone`; only the line/marker use `#43efa9`, while financial text retains
  its contrast-safe color. Added regression assertions for that separation.
- Removed play/pause buttons, translations, unreachable manual-pause state
  and obsolete pagination scroll handling. Retained automatic rotation,
  directional gestures, keyboard/range selection and context-based pauses.
  Pagination now uses the selected 8px gap and 20px track.

Verification after the final chart-color fix:

- `npm --prefix mobile run release:gate`: 800/800, both type checks,
  PWA/Tauri builds, artifact/bundle/source/test-quality budgets passed.
  Final log: `/tmp/home-refine-gate-final.log`.
- Earlier focused Home/return/layout selection: 24/24.
- Ego TaskSpace 11: all 16 combinations of 320/360/390/448px,
  light/dark, guest/member had no horizontal overflow or heading/chart/copy/action
  overlap. Guest/member cards measured 128px; actions 78x30px with 6px gaps;
  slides 108px; pagination 20px; endpoints 8x8px. No broken images or guest
  private wallet requests. Both themes resolve the positive plot to mint.
- Eight further English combinations at 320/448px, light/dark, guest/long-amount
  member had no overlap or document overflow. All digits of
  `9,007,199,254,740,993.01` were retained. English copy and long amounts may
  expand the card height instead of clipping.
- Real browser timer checks confirmed automatic advance, hover pause longer
  than 5500ms and resume on leaving, range End/ArrowLeft selection through
  12 slides, focus pause longer than 5500ms, and reduced-motion pause.
- Real touch events advanced a slide without navigating; the eye button hid
  the balance and chart. Empty configuration hid the region, one slide omitted
  pagination, and a request failure exposed retry without a fabricated slide.
- The initial automation wait targeted a transparent input as visible and
  timed out. Inspection showed the input existed and the carousel was advancing;
  focusing it and sending real keyboard events passed. This was a test wait
  mismatch, not an application load failure.

Viewed screenshots:

- `/tmp/home-refine-final-light-guest.png`
- `/tmp/home-refine-final-dark-guest.png`
- `/tmp/home-refine-final-light-member.png`
- `/tmp/home-refine-final-dark-member.png`
- `/tmp/home-refine-320-en-guest.png`
- `/tmp/home-refine-320-en-long.png`

This follow-up changes Mobile presentation and related checks/docs only.
Backend and Admin results above are prior verification, not rerun claims.
The existing Admin bundle blocker remains open; no budgets were raised.
TaskSpace 11 was finished once with no retained pages. The existing 5182/5184
preview servers remain running. No commit, push or deployment was performed.
