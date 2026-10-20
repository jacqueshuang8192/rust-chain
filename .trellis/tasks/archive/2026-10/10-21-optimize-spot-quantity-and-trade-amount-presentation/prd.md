# Spot quantity and turnover presentation

## Goal
Make Mobile spot quantity and quote amount readable without changing submitted decimal values.

## Requirements
- Unfocused quantity shows grouped maximum eight decimals; quote amount follows existing asset display precision (USDT two decimals, crypto eight).
- Focus restores exact decimal text; keep quote input draft stable while editing, including trailing decimal points. Blur only changes presentation, never order quantity.
- Reuse financial display helpers; tiny nonzero values remain distinguishable from zero, invalid drafts remain editable. Clear percentage selection on manual edits.
- Keep unit separated and full raw value available as title. Contract forms, price input, matching and backend stay unchanged.

## Acceptance
Behavior tests verify screenshot values, focus/blur preservation, quote editing, tiny values and unsafe integers. Mobile release gate passes without increasing budgets.

## Plan
Inspect exact conversion path; connect display and editing states; validate and record.
