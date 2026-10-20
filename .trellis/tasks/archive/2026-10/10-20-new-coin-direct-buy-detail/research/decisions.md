# Implementation and visual decisions

- Codex inline task, no delegated work. Existing main workspace was clean except
  `.vite/`; preserve it. No new migration, deployment or remote push.
- Direct purchase reuses the mature trading settlement inside one transaction,
  with a distinct canonical operation and response wrapper carrying the key.
- Root cause of valuation: assets depended solely on the correctly filtered spot
  catalog. Add wallet metadata; do not undo listing gates. PC also omitted frozen
  and locked amounts from value, so count all booked buckets once.
- App-specific design scope: `design-taste-frontend` explicitly excludes app
  screens. Apply hierarchy, spacing, readable type and screenshot critique to the
  trading interface; do not impose its marketing photography/motion quotas.
- Real content inventory: KHB identity/logo, five actual stages, fixed subscription
  price, supply progress, live chart, immediate buy, owned holdings, return,
  original cost, unlock terms and fee. These determine section order.
- Baseline screenshot: first screen was consumed by duplicate stage/progress
  content and a very tall chart; action below fold; holdings had weak hierarchy.
  Fix three priorities: action visibility, compact chart, readable holding value.
- Final layout: compact identity/price/supply, readable stages, purchase action,
  chart, value/return summary with native detail disclosure, issuance rules.
  Keep existing brand, themes and locales. Do not restore deleted notices.
- Fresh reload required for final browser acceptance: Vite HMR briefly retained
  old templates despite new CSS. Final screenshots and checks use fresh modules.
- Bundle budgets remain unchanged. Remove unreferenced token copy and obsolete
  stage-header CSS; explicit token-control/token-data selectors replace repeated
  structural selectors without touching chart descendants.
