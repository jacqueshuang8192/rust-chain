# Home portfolio typography and illustration

## Goal
Reduce Home portfolio balance typography, format it with Numeral, and replace the return-history chart with a suitable transparent illustration.

## Requirements
- Mobile Home only. Retain exact wallet valuation, today return, privacy toggle, loading/error semantics, and guest layout.
- Reuse the existing precision-safe Numeral helper. Preserve the existing four-decimal display rounding and tiny-nonzero preservation; group digits and remove trailing zeros without unsafe monetary Number conversion.
- Normal amount font 26px, long amount 22px with wrapping at narrow widths.
- Replace historical chart with a decorative transparent image, ignored by assistive technology. Remove unused Home chart state, requests, and styles; retain shared history API/domain and its tests.
- Built-in generation succeeded on final continuation; use the generated RGBA wallet illustration in mobile/src/assets/home/portfolio-wallet.png.

## Acceptance
- Total assets, today return, privacy and refresh behavior retained.
- No Home return-history requests or chart remain.
- Decorative image has actual alpha, no rectangular backdrop, fits light/dark 320–448px layouts.
- Mobile release gate and browser behavior/layout checks pass without changing budgets.

## Out of scope
PC, backend, other charts, production deployment and unrelated Cloudflare investigation.
