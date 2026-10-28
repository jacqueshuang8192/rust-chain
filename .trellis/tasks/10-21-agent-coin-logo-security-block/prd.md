# Diagnose agent coin logo upload security block

## Goal
Restore agent coin logo uploads after identifying the actual security blocking rule.

## Evidence
- User supplied Cloudflare block HTML for cllbmz.kdns.fr, Ray ID a4d96b139804b7fd.
- Frontend posts FormData with file to /agent/api/v1/coin-applications/logo; client preserves browser-generated multipart Content-Type.
- Agent route authenticates and delegates to configured shared upload storage.
- Targeted CoinLogoUpload and API client tests pass: 25/25.

## Open questions
- Failed request URL and HTTP status; whether this domain is the application origin or storage endpoint.
- Cloudflare security event matching the Ray ID: service, action and rule ID.

## Requirements
Identify the matched rule before changing application code or edge configuration. Preserve authentication and upload validation. Limit remediation to the affected route and confirmed rule.

## Acceptance criteria
- [x] Identify upload route and validate existing multipart behavior.
- [ ] Obtain security event and establish blocking layer.
- [ ] Apply evidence-based remediation and verify real upload success.

## Out of scope
Global WAF disablement, unrelated issuance changes, deployment without evidence.

## Status
Investigation only; no production code or Cloudflare configuration changed. Official documentation lookup returned no usable results this session.
