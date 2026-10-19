# Final administrator browser verification — 2026-10-19

Ego Browser task space 23, local Vite app on 3030. All API traffic was intercepted by in-memory fixtures; no real issue, allocation or buyback was submitted. Screenshots were opened with the local image viewer.

- Creation: selected HIP/USDT; supply 1,000,000; fixed subscription 0.2; purchase seed 0.3; listing reference 9.99; automatic subscription/purchase and manual distribution/listing; fixed unlock 2026-11-20 11:00 Asia/Hong_Kong; positive-profit fee 20%. Confirmation at 1728/390/320 contained the complete policy without document overflow. UTC payload matched local timestamps. Simulated lost response then retry sent exactly two identical bodies with the same key and resolved to project 701.
- Holdings: 51-row fixture requested `limit=50&offset=50&project_id=7` (query order immaterial) when moving to the second page. Saved CSV contains the single loaded row 851. Error and empty states disable export; refresh/reset restores the list. Original-policy modal loads shared CSS and shows the actual-purchase lock and 20% positive-profit fee. No write calls.
- Holdings geometry: document widths 1728/1280/390/320 exactly match viewport; table containers 1414/1022/277/207 remain horizontally contained.
- Buybacks: original cost 500, gross 1000, fee 100, net 900 USDT; original quote source/version displayed with Chinese labels. Record page and detail drawer remain contained at 320; page widths 1728/1280/390/320 pass. No write calls.
- Operations: actual `scheduler` actor is displayed as 自动调度; stage operation and original JSON evidence retained. Page widths 1728/1280/390/320 pass. No write calls.
- Project: issuer AGENT-A is distinct from BUYER-B; actual purchase is absent; planned automatic node, retry time and blocked state are visible. Contract is read-only; project-scoped links address holdings/buybacks/operations. Contract page fits 320.
- Market: fixed HIPUSDT selector is disabled; initial price is 0.3 and disabled. Before actual purchase, publication is disabled with an explicit explanation. Market page widths 1728/1280/390/320 pass. No write calls.
- Permissions: read-only project/market permissions remove phase/save/publish/pause controls. Visiting operations without `audit.logs.read` routes to `/403`, performs no operations read and no writes.

Final inspected artifacts include `holdings-final-1728.png`, `holdings-final-320.png`, `holding-rules-final-320.png`, `buybacks-final-1728.png`, `buyback-detail-final-320.png`, `operations-final-1728.png`, `operations-final-320.png`, `project-overview-1728.png`, `project-contract-320.png`, `project-market-1728.png` and `project-market-320.png`, under `/tmp/admin-token-qa-20261019/`.

Test harness adjustments: ambiguous duplicate text in the table/drawer was resolved with a scoped dialog query; the permission-denied page's actual copy is 无权限. Neither issue was an application failure. Final receipts are `creation-results.json`, `holdings-final-results.json`, `buybacks-final-results.json`, `operations-final-results.json`, `project-final-results.json`, `market-final-results.json`, `readonly-final-results.json`.

Space 23 was closed with `finish({keep:[]})`; no retained preview tabs.
