# Market feed versus spot listing

Purchase activation in new_coin/infrastructure/issuance.rs intentionally makes
the fixed trading pair active so default generation and live token quotes work.
Listing keeps this pair and engine and transfers remaining supply. Disabling the
pair or filtering symbol-specific feed reads would break purchase charts.

Market list and favorite queries used only active status. Financial order
insertion already checked lifecycle, but rule loading and compatibility writes
did not; actual listing evidence was not required. New predicates require both
listed and non-null actual_listed_at for v2, preserving legacy NULL policy.

Mobile snapshot merge already replaces membership, except the store threw on an
empty array. API now distinguishes a real empty catalog from bad/all-failed
quotes. PC appended unknown live symbols, bypassing REST membership. New-coin
opportunities previously required only a matching pair ID, allowing stale cache
to expose purchase projects; now they also require listed stage.

Verification uses local MySQL 9.3 database codex_spot_listing_20261020 and a
standalone loopback Redis on 16391. No production systems are used. Mongo is not
available; lifecycle checks only prove purchase K-line requests reach the existing
storage boundary. Price, pair, generator continuity and quote settlement are
exercised through real MySQL/Redis routes.
