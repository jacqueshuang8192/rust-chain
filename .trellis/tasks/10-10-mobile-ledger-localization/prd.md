# Mobile Ledger Type Localization

## Scope

Fix transaction ledger metadata displaying raw known change-type codes.
Existing translations include seconds_contract_settle_win; the view incorrectly
falls back to the raw code for known types.

## Acceptance

- Known types use existing translations in Chinese and English, including metadata titles.
- Unknown types retain their source value for diagnosis.
- Amounts, filters, API data and ledger classification remain unchanged.
- Run focused regression tests and the mobile release gate; record results.

## Approach

Use the existing presentation mapping and translation function in the view.
Exercise the production view functions with both locales and every known type.
Prior market-label changes remain separate and uncommitted.
