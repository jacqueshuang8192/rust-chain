

### Tron free-provider qualification

1. **Scope:** Tron offers five distinct documented free providers; limited trials and paid-only templates never contribute to that free count. This remains setup coverage, not healthy-channel count.
2. **Signatures:** optional browser-only `NodePreset.account.freeTier: { quota, source, checkedOn }`; `providerChoiceCount(network, publicPresets, freeOnly = false)` filters account templates by explicit qualification and deduplicates public/account aliases.
3. **Contracts:** free counts derive from the supplied server catalog; an older server without `tron-publicnode` reports four, not a fabricated fifth. Show quota units accurately (credits/CU/RU/requests), registration and official source/date. Keep `SaveScannerProvider` unchanged.
4. **Validation:** freeTier absent/trial -> excluded; PublicNode missing/nonrecommended -> excluded; TronGrid public plus account -> one provider; empty key/endpoint, revision conflict and cancellation retain the established behavior.
5. **Cases:** good: append public defaults with fresh anonymous slots while preserving an existing private key; base: no saved changes until confirmation; bad: count multiple keys as independent free providers or equate free with unlimited throughput.
6. **Tests:** assert five/legacy-four free choices, paid exclusion, source links, unchanged credential DTO, append-once and conflict retention. Check desktop and 320/390px disclosure/save flows.
7. **Wrong/correct:** wrong: silently upgrade a custom row to an anonymous URL; correct: explicit catalog selection/save and authoritative runtime health.
