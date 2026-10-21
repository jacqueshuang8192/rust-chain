# Reference-style spot holdings

## Requirements
- Restyle Mobile spot holdings as flat rows: asset header, three columns for equity quantity, unavailable cost price and latest USDT price, then available balance and frozen/locked details. Secondary USDT valuation when available.
- Hide USDT rows only in spot holdings; preserve spendable USDT for orders and Assets page.
- Wallet API lacks cost basis/PnL: cost remains em dash; do not invent returns or expose unsupported actions from reference.
- Preserve current-pair filtering and state/navigation. Exact decimal totals and issuance valuation freshness apply; missing price stays unknown.
- Validate real data behavior, narrow light/dark layouts and Mobile release gate, unchanged budgets.
