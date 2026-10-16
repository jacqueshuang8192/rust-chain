# Agent header identity

## Goal
Replace the meaningless authentication subject in the agent header with verified account details from the existing GET /agent/api/v1/me endpoint.

## Scope and acceptance
- Show the exact username, a Chinese agent level, and the agent code with a clear label.
- Use existing Semi Design tokens and a compact two-line identity layout. Long text must truncate without displacing navigation or logout at 320/390/1280/1728px.
- Never expose the internal auth subject as a display fallback. Loading and failure states use neutral Chinese copy; failures have an explicit retry action.
- Scope identity state to the subject and session generation. Cancel requests on unmount and ignore late responses after a session change.
- Validate display fields at the API boundary; preserve existing dashboard callers.
- Keep authentication behavior, backend APIs, and unrelated pages unchanged.

## Plan
1. Add cancelable identity loading and header presentation.
2. Cover real identity, errors/retry, malformed API responses, and account-switch races.
3. Run Web checks and local browser fixture verification; record results and any existing gate failures.

## Delivery
Implemented and verified. Header now displays the API username, Chinese agent level, and labeled code. Loading/failure/retry and generation-scoped cancellation are covered.

- Web full suite: 1083/1083; production policy: 15/15; coverage suite: 23/23. Typecheck, lint, build, and diff checks pass.
- Existing total JavaScript budget still fails: 2,676,877 bytes against 2,600,000 (previously 2,675,592); threshold unchanged.
- Local read-only API fixtures in Ego Browser: normal/long identity at 1728/1280/390/320px; no overflow or header action overlap. Failure/retry, mobile menu, and logout pass. Final desktop/mobile screenshots inspected.
- No live authenticated backend account, full unrelated admin-page matrix, or physical device verification; no backend contract changes.
- User authorized committing and pushing this task to origin/main on 2026-10-17. No deployment. Browser fixture task space closed; existing development server retained.
