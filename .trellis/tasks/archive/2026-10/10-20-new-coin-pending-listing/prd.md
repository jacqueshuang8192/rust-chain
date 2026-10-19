# Align mobile new-coin stages with the deployed issuance flow

## Goal

Investigate the reported pending-listing state and correct the Mobile wording,
stage guidance and stale project state. The user confirmed the issue is on the
deployed KHB detail page and requested corresponding frontend changes.

## Confirmed evidence

- On 2026-10-20 01:26 Asia/Hong_Kong, the public KHB response identified project
  2 as version 2, active, in `distribution`, with 100 reserved and 0 allocated.
- Automatic purchase was scheduled for 2026-10-20 00:20; listing is manual at
  2026-10-21 04:56. Pending subscription obligations block purchase/listing.
- The fresh deployed detail shows waiting distribution. The list calls that
  same phase pending listing; its existing purchase filter precedes distribution.
- Detail fetches the project once; the holdings refresh does not reload project
  lifecycle. Scheduled timestamps cannot substitute for actual lifecycle state.

## Requirements

- Keep Mobile list/card/detail phase terminology and order consistent: preheat,
  subscription, distribution, live purchase, listing. Preserve legacy v1 flow.
- Provide short contextual next-step guidance and a records entry during
  distribution. Do not assert a particular user's allocation from aggregate data.
- Label scheduled times as planned. Retain the user's prior removal of timezone,
  automatic/manual switches and the long allocation/unlock explanation.
- Project cards must display the v2 contract's actual lock rule, not the legacy
  compatibility fields (KHB has a fixed unlock but the old card said on listing).
- Refresh detail lifecycle while visible and on return to the page, support
  explicit project refresh, and ignore late reads after route change/unmount.
- Do not replace a reviewed legacy transaction during background refresh.
- Preserve v2 subscription receipt/retry recovery when the project advances;
  closed subscription phases must not accept a new intent.
- Do not change backend settlement, agent permissions, node switches or funds.

## Acceptance

- [x] No distribution filter or timeline is labelled pending listing.
- [x] v2 has five correctly ordered stages; v1 has four and no synthetic purchase.
- [x] Planned deadlines never advance the displayed stage or unlock actions.
- [x] Project refresh moves from distribution to purchase/listed from API data,
  stops when hidden/unmounted, and cannot apply another symbol's late response.
- [x] Pending subscription recovery survives phase changes; no new intent after close.
- [x] Focused behavior tests, Mobile release gate and browser checks pass.

## Scope / delivery

Mobile frontend and its tests/specs only. Production observation is read-only;
validation uses local in-memory fixtures. No remote push, deployment, stage
mutation, distribution or real financial transaction is part of this task.
