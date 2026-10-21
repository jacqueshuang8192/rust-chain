# Mobile spot holdings presentation

## Scope
Improve existing current-pair holdings cards: asset identity/icon, exact total holdings, labelled available and frozen/locked balances. Reuse walletTotal and existing financial presentation. Group numbers by asset display precision, retain tiny nonzero thresholds and raw amount titles. Wrap large values; compact cards do not stretch to fill empty panel space. Preserve filtering, auth/loading/error/empty states and navigation. No invented cost/PnL, new requests, backend or PC changes.

## Acceptance
Exact available+frozen+locked counted once. Existing wallet/financial tests and Mobile release gate pass; browser actual card checks confirm layout and amounts.
