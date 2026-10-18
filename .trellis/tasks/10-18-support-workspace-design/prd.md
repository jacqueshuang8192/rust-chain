# Online support workspace refinement

## Goal
Optimize the shared agent/admin customer support interface using the requested design-taste-frontend principles and existing app conventions.

## Requirements
- Replace the horizontally scrolling conversation table with a compact inbox: customer, contact, preview, time, ownership, status and unread count remain available.
- Desktop: coordinated inbox/chat split, independent scrolling, readable message bubbles and a reachable composer.
- Mobile: list first, select to open conversation, return without losing the current reply draft.
- Preserve admin-only ownership/unread filters, exact-agent visibility, server pagination, current-page search, polling, read/status actions, history pagination and idempotent retries.
- Keep scoped styles, Chinese copy, visible keyboard focus and reduced-motion behavior. No new dependencies or backend changes.

## Design direction
An operational inbox: neutral surfaces, clear typography, an obvious selected conversation and generous chat space. Keep existing shell and brand fonts. The named skill explicitly excludes dashboards/app screens; apply its applicable hierarchy, flat-surface and screenshot critique principles to this existing component, without marketing hero, stock pictures or decorative animation quotas.

## Subject inventory and content argument
HIPPO customer support; exact agent ownership; administrator unassigned queue; unread customer requests; current-page search; durable message history; retry identity; closed/reopened conversations; Chinese operational copy; customer contact details.
Queue proves which customer needs attention. Conversation header identifies who and which ownership. History preserves context. Composer provides one clear reply action. These are existing application data, never fabricated production statistics.

## Acceptance
- [x] Both scope layouts usable at 1728/1280/768/390/320px without document overflow.
- [x] Existing automated filter, server pagination, read/status, retry and history checks pass.
- [x] Mobile list return retains draft and selection; message area and composer do not overlap.
- [x] Targeted/full web checks and browser fixtures run, limitations recorded.
- [x] Spec/progress records updated.

## Plan
1. Inspect existing implementation and capture baseline.
2. Recompose shared workspace and scoped styles; update necessary interaction checks.
3. Browser critique and refinement; quality gates and progress record.

## Out of scope
Global shell redesign, backend routing/permissions, unrelated coin tabs from the previous turn, production messages, push/deployment.

## Validation and remaining work
- Web full suite: 113 files / 1086 tests passed; production policy 15/15, coverage 23/23 and thresholds passed.
- Final small layout adjustment: targeted support/API 19/19, typecheck, lint, production build and diff check passed.
- Design source lint: 0 FAIL / 0 WARN. Three documented exceptions are false positives for `box-shadow: none`, which removes inherited shadows.
- Bundle budget: still fails at 2675427 / 2600000 raw JavaScript bytes (previous turn baseline 2677241); threshold unchanged.
- Baseline captured and inspected at /tmp/support-before-desktop.png using only local in-memory fixtures.
- User approved replacement of the vanished Ego space. Final two-scope browser checks passed at 1728/1280/768/390/320px: queue/chat overflow, composer reachability and non-overlap. Draft retention, same-ID send retry, long text, empty search, cached error state and read-only controls passed with local in-memory fixtures. Space 9 was closed.
- Final screenshots inspected for both scopes at desktop/tablet/mobile widths. No production messages, push or deployment. Support work is prepared as an isolated local commit; prior coin-application tab changes remain untouched.

- Final visual refinements (heading size, single character counter, responsive wrapping): support/API 19/19, typecheck, lint and build passed. Full suite before these presentation-only refinements was 1086/1086.
- Live authenticated design probe/judge used the skill inspector modules at 1440/768/390px. Full-page marketing checks still flag unchanged global shell decoration, compact metadata type and absent marketing imagery/hero; no full design-gate pass is claimed. See design-review.md.
