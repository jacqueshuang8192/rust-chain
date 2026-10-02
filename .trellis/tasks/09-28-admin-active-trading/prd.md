# Administrator Active Trading Views

## Scope
Expose existing administrator seconds orders and margin positions as clear active views. Preserve existing read permissions, all-record access, pagination, exact monetary text and detail actions. No financial mutations, database migrations or unrelated dirty changes.

## Plan
- [x] Default seconds orders and margin positions to opened (the administrator API status), with explicit all-record tabs.
- [x] Reuse agent countdown for administrator seconds orders; expiry means awaiting settlement, not a guessed terminal result.
- [x] Display authoritative position fields; preserve existing manual refresh and detail interactions, retaining fixed status during filter reset.
- [x] Verify components, routing, permissions, types and build; record limitations. Do not commit without user authorization.
- [x] Release gate: feature verification is complete; the existing bundle budget overage remains an external release blocker.

## Acceptance
Existing URLs and permissions remain valid. Active/all switching resets paging; active status cannot be removed by filter reset. Countdown only applies to opened orders. No per-row risk polling or fabricated market/PnL values.

## Confirmed Query Boundary
The existing administrator API includes unfilled limit orders in `opened`.
Current positions must request `status=opened&filled_only=true`, applied to both
the SQL row query and count. Omitted/false preserves existing all-record behavior.
All-record rows with opened status and missing entry price display as pending,
not as risk-bearing positions. No transaction or settlement behavior changes.
