# Default Seconds Entry To BTC

Bare `/seconds` and legacy `/products/seconds` must select BTC/USDT regardless
of saved choice or API list ordering. Explicit symbol URLs still win and survive
refresh. If BTC is unavailable, use the first available product; empty/error
responses remain unchanged. Preserve pending picker choices and history behavior.

Implement in the existing selection composable, update behavior tests and the
navigation contract. Run the mobile release gate. No push or deployment.
