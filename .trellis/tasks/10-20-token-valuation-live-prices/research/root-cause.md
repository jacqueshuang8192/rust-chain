# Root cause and selected repair

The real default producer calls MarketTickerCacheEntry::from_snapshot with
MarketDataProvider::Strategy. The resulting Redis payload has source=generated
and provider=strategy; load_latest_price normalizes it to strategy. Issuance
engine_ready requires an exact match with market_pair_generation_runs.active_source,
which is default. Both holdings and wallet therefore return null. The earlier
lifecycle fixture hand-authored source=default, hiding this integration mismatch.

Preserve generic public provenance. Resolve the private issuance source using
the existing one-row market_generation_price_checkpoints record matched to the
Redis price and observation time, plus the current runtime generation/version.
Do not infer source solely from current config and do not fall back to SQL price.
Keep freshness, pause, lease, quote confirmation identity and spot gates intact.
Regression tests must serialize the real producer DTO and exercise default
ingestion into real isolated MySQL/Redis before reading both authenticated APIs.

## Retrospective

Categories: cross-layer contract mismatch (B), test coverage gap (D), and implicit
assumption (E). The previous test fixture wrote source=default directly, so it
made the consumer's incorrect identity assumption appear valid. Switching only
that fixture to the real producer DTO reproduced the missing value before the
repair. Future financial display coverage must include an actual publisher and
authenticated consumers, plus stale-engine and source-switch negative cases.
