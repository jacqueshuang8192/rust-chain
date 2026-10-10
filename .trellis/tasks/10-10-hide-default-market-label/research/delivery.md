# Market Notice Visibility

## Changes

- Mobile/PC shared labels suppress default-generated and external-reference
  source text in both locales. Label-only uses omit the entire empty element.
- Ticker observation time, unknown-time and stale indicators remain visible
  without a leading separator; other source labels retain their behavior.
- Mobile chart no longer renders the generated-market notice or its 44px strip.
- API fields, source classification, locale dictionaries, prices, chart points,
  history/live updates and all trading/settlement behavior are unchanged.

## Verification

- Mobile focused label/chart render tests: 12/12 passed.
- `npm --prefix mobile run release:gate`: passed, including application/test
  type checks, all tests, PWA/Tauri builds, artifact checks and both budget gates.
- PC `node --experimental-strip-types --test tests/*.test.ts`: 119/119 passed.
- PC `type-check` and `build`: passed.
- `git diff --check`: passed.
- Ego TaskSpace 4: mounted actual shared components with local fixture data.
  Mobile tested 390/320px, PC label and OrderBook tested 1280/320px. Both locales
  and default/external/platform/unknown source transitions were exercised.
  Hidden labels produced zero `small` nodes in label-only/book positions;
  freshness and other labels remained. Document overflow was zero.
- Mobile chart remained ready with 120 fixture candles and 27 sampled canvas
  colors, no notice node. Screenshots inspected at `/tmp/market-notices-*.png`.
  Component checks blocked XHR attempts and are not live market integration.
- TaskSpace closed once. Local dev servers remain on ports 5186 and 5187.
  No order submission, deployment, commit or push performed for this task.

## Proposed Commit

`fix: hide requested market source notices`

Include only:

- `mobile/src/components/{MarketProvenanceLabel,MobileMarketChart}.vue`
- `mobile/tests/{market-generated-source,market-provenance-label}.test.ts`
- `pc/src/components/trade/MarketProvenanceLabel.vue`
- `pc/tests/market-provenance.test.ts`
- `.trellis/spec/backend/market-provenance.md`
- `.trellis/tasks/10-10-hide-default-market-label/`
- This task's entry in `docs/superpowers/PROGRESS.md`

Exclude the existing `.vite/` cache. Await confirmation before committing.
