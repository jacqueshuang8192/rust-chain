# Home Carousel Contract

## 1. Scope / Trigger

Apply when changing the Mobile Home promotion area, the Admin carousel editor,
its public/configuration APIs, or migration `0146_home_carousel.sql`.
The carousel is independent of market data and must never manufacture account,
return, market, or campaign values when the configuration is empty or unavailable.

## 2. Signatures

- `GET /api/v1/home/carousel?locale=zh-CN` is anonymous.
- `GET /admin/api/v1/home/carousel` requires `content.news.read`.
- `PATCH /admin/api/v1/home/carousel` requires `content.news.write`.
- Admin UI: `/admin/home/carousel`, under Content Operations.
- Image uploads reuse `POST /admin/api/v1/uploads/images` and its separate
  `system.uploads.write` permission. Carousel write access does not grant upload access.
- `home_carousel_configs` has one row (`id = 1`), unsigned `revision`,
  JSON `slides`, nullable `updated_by_admin_id`, and `updated_at`.
- The JSON column stores `{ "slides": [...] }`, not a bare array.
- `HOME_CAROUSEL_TEST_DATABASE_URL` enables the isolated MySQL route test.
  It is a test-only setting, not an additional production connection.

## 3. Contracts

```json
{
  "expected_revision": 0,
  "reason": "Publish the approved invitation artwork",
  "slides": [{
    "id": "invite",
    "position": 0,
    "enabled": true,
    "locale": null,
    "title": "Invite friends",
    "description": "View your invitation records.",
    "image_url": "https://cdn.example.test/invite.png",
    "target": { "type": "referrals", "id": null }
  }]
}
```

- Admin GET/PATCH return `{ revision, slides }`; public GET returns only
  `{ slides }`, with no revision, administrator, timestamp, or audit metadata.
- At most 12 slides, including disabled items. IDs match
  `[A-Za-z0-9_-]{1,64}` and are unique. Positions are strictly increasing,
  unique integers in `0..11`; the editor serializes current order as `0..n-1`.
- Title/description contain 1..80 / 1..180 Unicode characters after trimming.
  Image URLs are at most 2048 Unicode characters, HTTP(S), with a hostname and
  without embedded credentials. Clients may impose a stricter UTF-16 limit.
- Locale is null (all languages) or `[a-z]{2}(-[A-Z]{2})?`; query omission
  defaults to `zh-CN`. Matching includes the language family (`en-US` / `en`).
- Targets are only `referrals`, `news_center`, and `news_detail`. Only the last
  requires an ID, a positive JS-safe integer. Other targets require null/no ID.
- Save requires a trimmed 1..512-character reason and expected revision below
  `9007199254740991`. Unknown request/document fields are rejected.
- Lock singleton row with `SELECT ... FOR UPDATE`, compare expected revision,
  update, insert `home_carousel.save` audit, and commit in the same transaction.
  Audit before/after use identical `{ revision, slides }` shapes.
- Public reads exclude disabled items, unmatched languages, and news targets
  that are not currently published. Empty results are valid, with no fallback
  campaign. Corrupt stored JSON must fail, never turn into an empty editable draft.
- The editor preserves its draft revision through background refresh and 409.
  It does not silently replace the expected revision to overwrite another editor.
- Mobile uses `publicApiRequestConfig` without Authorization, validates unknown
  API data, and navigates only to typed internal routes. No runtime asset/import
  may depend on `mobile/pencil`.
- Mobile rotates every 5500ms, pausing on focus, hover, touch, hidden document,
  or reduced motion. The approved design has no play/pause buttons; do not add
  them or retain an unreachable manual-pause state. Compact dots share one native
  range input on a 20px rail, separated from the slide by 8px.
- Guest actions match the selected 78x30px faces, with 6px vertical spacing.
  The guest copy uses 4px gaps, an inset card border and theme-specific shadow.
  The member heading resets inherited prototype gaps/padding: use a 6px
  heading gap and a 4px return-row gap, with no extra return-row margin.
- Keep the return line linked to real geometry. The endpoint is an 8px HTML
  marker positioned from that geometry over the plot, not a circle stretched
  non-uniformly by the SVG viewBox. Positive plot strokes use the mint fill role;
  financial text retains its contrast-safe positive role. Plot tones use
  `data-tone`, not the prototype's `.positive`/`.negative` classes, whose
  `!important` text colors would override the chart palette.
- Locale replacement and unmount invalidate requests; disposal removes timers
  and listeners. Vertical gestures do not change slides; a swipe suppresses
  its trailing click. Image failure preserves the artwork dimensions.

## 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| Anonymous Admin request | 401, no mutation |
| Read-only Admin PATCH | 403, no mutation |
| Invalid domain field/reason/locale | 400, entire save rejected |
| Malformed JSON or unknown JSON field | Axum extraction rejection before use case |
| Stale revision / two saves with same revision | 409; exactly one concurrent save succeeds |
| Audit insert fails | 500; configuration and revision roll back |
| Missing pool / corrupt stored document | 500; no fabricated empty state |
| Missing singleton row | 404; do not silently recreate |
| No eligible public slides | 200 with empty array; Mobile hides carousel |
| Transport/contract failure on Mobile | Explicit retry state; no demo content |

## 5. Good / Base / Bad Cases

- Good: editor A saves revision 4 as 5; editor B still submits 4, gets 409,
  and keeps its unsaved content.
- Base: a fresh migration creates revision 0 with no slides. The Home page
  remains usable and makes no private wallet requests for a guest.
- Bad: presenting an unpublished news item, accepting an arbitrary URL as a
  navigation target, or showing a sample campaign when the API fails.
- Base: the live return chart can be a straight line when there are only two
  real points; the Pencil illustration is not financial evidence.

## 6. Tests Required

- `tests/unit_src/src_modules_home_carousel_domain_tests.rs`: normalization,
  Unicode limits, target/URL/ID/revision-compatible boundaries, locale and order.
- `tests/openapi_routes/home_carousel.rs`: OpenAPI and anonymous boundaries;
  actual MySQL migrations run twice, singleton constraint, independent read/write
  permissions, filtering, 409, concurrent save, audit contents/failure rollback,
  and clearing configuration.
- `web/src/admin/actions/HomeCarouselPage.test.tsx`: actual Semi editor,
  add/remove/reorder/disable, reason, exact PATCH, revision, read-only and 409.
- `mobile/tests/home-carousel.test.ts`: strict adapter, public request config,
  stale response isolation, timers/disposal, directional swipe and click suppression.
- Run Rust check/clippy/fmt, library/OpenAPI/architecture/documentation tests,
  Mobile `release:gate`, and the full Admin quality gate. Report existing
  bundle-budget failures separately, without increasing thresholds.
- Browser: 320/360/390/448px, light/dark, guest/member, English, long amounts,
  empty/error/single/12 slides, actual autoplay/context pause, keyboard range, privacy,
  image fallback, and desktop/390px Admin editing/confirmation/upload callback.
  Isolated fixtures are not production end-to-end evidence.

## 7. Wrong vs Correct

```text
Wrong: read version outside lock -> update config -> commit -> write audit.
Correct: lock row -> compare draft version -> update + audit -> commit.

Wrong: import Pencil assets or use hardcoded promotions when API returns [].
Correct: tracked production artwork; backend-owned campaign URL/content;
         hide the carousel for [] and expose retry for transport failure.
```
