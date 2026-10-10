# Hide Requested Market Notices

## Goal

Remove the default-generated/external-reference source labels and the Mobile generated-market
chart notice, as explicitly requested, without changing market data.

## Scope

- Mobile and PC shared provenance labels hide the `default` and `external` source text.
- Hide an otherwise empty label node so books/trades have no blank row.
- Keep ticker observation time and stale/unknown-time indicators independently.
- Remove the Mobile chart's generated-market notice and its reserved spacing.
- Retain source/provider/market-type metadata and other source labels.
- Keep pricing, generation, settlement, chart history and live updates unchanged.
- Retain locale keys; do not replace translations with empty strings.
- No backend changes, dependency changes, deployment or automatic push.

## Plan

1. Update the shared display components and remove notice-only chart styling.
2. Add label regressions and update the actual shared chart render test.
3. Run Mobile release gate, PC tests/typecheck/build and local browser checks.
4. Record verification, update the provenance presentation spec, propose commit.

## Acceptance

- [x] None of the three requested notices is rendered in its affected UI.
- [x] Default/external sources remain in data; other labels and freshness remain intact.
- [x] No blank hidden-source label or chart notice spacing remains.
- [x] Relevant tests, build/type gates and browser results are recorded.
