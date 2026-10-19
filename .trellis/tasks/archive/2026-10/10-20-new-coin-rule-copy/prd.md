# Mobile new-coin rule copy

## Scope

Only the existing Mobile new-coin issuance-rule presentation, following the user's three explicit requests. No backend, price-field, lifecycle, lock/unlock calculation or other portal changes.

## Acceptance

- [x] Display `发行价` instead of `行情初始价`, keeping `purchase_initial_price` as the value source.
- [x] Remove the listing-reference-price row from the page; retain the policy data contract.
- [x] Display `解禁规则` instead of `锁仓规则`; preserve its existing details.
- [x] Keep English labels aligned and pass the required Mobile release gate.

## Plan

1. Update the existing component and local translations.
2. Run the Mobile gate and inspect the scoped diff.
3. Record delivery in PROGRESS. No deployment is part of this copy change.
