# Priority multi-country configuration consistency

## Goal
Implement the first three fixes approved after the multi-country review: preserve product translation identity, prevent unusable KYC country configurations, and load complete admin country options.

## Plan and Acceptance
- [x] Preserve existing loan/earn content locale when reopening or changing country defaults; offer explicit locale selection without modifying text.
- [x] Reject conflicting KYC country allowlists/document rules on save; clients show the effective intersection for legacy configurations. Preserve empty-list semantics and existing country identities.
- [x] Share bounded country pagination across product/news options and KYC; cover more than 100 countries, errors and cancellation without displaying incomplete directories.
- [x] Run focused and cross-layer checks, record actual results and update progress.
- [ ] Release acceptance: resolve existing bundle-budget and backend documentation failures; complete real-API browser acceptance. Implementation is delivered, not deployment-ready.

## Boundaries
No historical data migrations, money operations, production configuration changes, automatic commits or pushes. Existing dirty work stays untouched. Review findings F4-F6 and regional payment/new-language expansion remain a later slice.

## Evidence
See ../09-26-multi-country-config-review/research/review.md.
Implementation results and explicit validation limits: research/delivery.md.
