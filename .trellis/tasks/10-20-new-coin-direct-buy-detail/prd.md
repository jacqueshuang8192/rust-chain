# Direct live token purchase and mobile detail redesign

## Confirmed scope
- In purchase stage, users enter token quantity and buy directly at the server's
  current valid market-engine price, without fetching/reviewing a buy quote.
- Restyle and reorganize the Mobile NewCoinDetailView (打新详情), following the
  explicitly invoked design-taste-frontend skill where applicable to this app.
- Preserve the existing brand, locales/themes, legacy issuance compatibility,
  subscription rules, holdings, lock/sale rules, automatic refresh and listing-only
  spot access. Selling keeps its existing fee-bearing quote/confirmation flow.

- Repair wallet valuation for pre-listing tokens independently of the spot catalog.
  Use fixed subscription price during subscription/distribution, valid engine price
  during purchase, convert the actual quote asset to USDT, and value only booked
  available/frozen/locked wallet balances (never pending allocation quantities).
  Missing/stale prices remain unknown. Apply the valuation fix to Mobile and PC.

## Implementation plan
1. Research current quote/confirmation/receipt transactions and client recovery;
   inspect the existing page in the browser and write a visual brief.
2. Add an authenticated one-request market purchase endpoint with fresh
   server-owned pricing, atomic settlement and canonical idempotent replay.
   No client execution price; duplicate/unknown-result retries must never buy twice.
3. Replace the buy-quote interaction with direct purchase and durable recovery;
   improve identity/current-stage, market chart, purchase form, rules and holdings
   hierarchy. Keep all real data and requested hidden copy removed.
4. Add wallet-only issuance valuation metadata and consume it without widening
   spot discovery or order permissions.
5. Exercise real isolated MySQL/Redis financial regressions, client behavior,
   required build/static gates, and browser narrow/theme/locale/state checks.
6. Update specs and PROGRESS, save local commits and archive; no deployment or
   remote push requested by this message.

## Acceptance
- One user action buys at a valid live server quote; no buy quotation step.
- Server rejects stale/unavailable engines, wrong phases, excess quantity or
  insufficient funds atomically. All amounts stay exact decimal strings.
- Successful retries return the original trade, even if price/stage changes;
  changed-intent keys conflict; uncertain UI outcomes stay recoverable per login.
- Purchase details prioritize live chart and primary action; planned rules and
  holdings remain readable at 320/390/448px, desktop shell, both themes/locales.
- No decorative demo content or fabricated market data in production.

## Decisions to resolve in code research
- Reuse existing quote receipt persistence inside one transaction for direct buys,
  with a distinct intent marker, if this preserves all execution invariants.
- Design-taste targets websites, explicitly excluding app screens: apply its
  hierarchy, spacing, typography and screenshot critique to the real trading UI;
  do not impose irrelevant marketing photography or large motion quotas.
