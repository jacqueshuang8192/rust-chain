# Diagnose Tron listener HTTP failure after restart

## Goal
Identify the cause of the reported Tron listener failure and repair the generic diagnostic that prevents operators from distinguishing upstream errors.

## Confirmed facts
- User reports a non-success HTTP status after restarting.
- User uses default TronGrid with an API key configured.
- Node::post discards status and operation; persisted last_error contains only a generic message.
- Saved API key is encrypted in the database and is read on each scan; Tron uses TRON-PRO-API-KEY.
- Local database inspection is unavailable; deployed response/status is not available in this workspace. No specific provider rejection cause is proven.

## Plan
1. Inspect node transport, restart/configuration and official TronGrid contracts.
2. Add safe actionable transport diagnostics, preserving statuses and code-defined operation names without secrets or raw upstream bodies.
3. Verify actual mocked HTTP paths, headers, success/error handling and privacy. Record limitations and operational steps.

## Acceptance criteria
- [x] Non-success HTTP responses identify status and requested operation.
- [x] Authentication, permission, rate-limit and endpoint errors have bounded actionable explanations.
- [x] Error paths do not leak endpoint, key, response body or redirected location.
- [x] Existing Tron request authentication and receipt parsing remain valid.
- [x] Targeted tests, format and production Clippy pass; progress and specs updated.

## Scope
Transport diagnostics and operator troubleshooting only unless further evidence proves an adapter defect. Do not reset cursors, change credit rules, lower confirmations, rotate keys, alter production configuration or silently skip blocks. User authorized commit and push on 2026-10-25. No deployment in this task.

## References
- https://developers.tron.network/reference/rate-limits
- https://developers.tron.network/reference/security-settings
- https://developers.tron.network/reference/gettransactioninfobyblocknum

## Validation and unresolved environment evidence
- Scanner unit tests: 12/12; includes actual HTTP mocks for status/operation, credential privacy, Tron header and successful JSON, and existing receipt parsers.
- Architecture: 11/11. Production Clippy --lib --bins -D warnings, cargo fmt and git diff --check passed.
- Local DB connection unavailable. No deployed logs or authenticated TronGrid response were read. Upstream documentation retrieval was attempted but direct requests were blocked; no provider-account diagnosis is claimed.
- Operational cause remains unconfirmed pending numeric HTTP status and failing method from the deployed environment. The verified diagnostic patch is being committed and pushed under explicit user authorization; deployment and operational recovery remain unverified. No cursor or financial state was changed.
