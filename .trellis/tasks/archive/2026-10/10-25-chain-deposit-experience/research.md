# Findings and design direction

- Admin ReceiptReview only accepts positive internal numeric IDs; customer-facing CD numbers leave its read button silently disabled. Baseline 3 tests pass but only exercise numeric IDs. Failed re-read retains the old eligible target; clear target before requests and surface exact blockers.
- Preserve Admin routes and mode selector; only repair receipt review.
- User panels in Mobile/PC are mirrored, combining draft, active order and paged history. Separate history into a component/page while preserving session aborts, exact decimal strings, persistent uncertain keys and server clock.
- Design inventory: HIPPO, five networks, precise coin quantity, 30-minute deadline, shared receiving address, original order number, immutable fees, confirmation depth, evidence attribution and late-payment handling. Payment receipt becomes the compositional device: prominent exact amount, independent deadline/status, QR/address payment block, compact immutable details. Records use chronological rows with amount/status, not repeated payment forms.
- The invoked taste skill explicitly excludes application/dashboard screens. Apply its clear hierarchy, flat semantic palette, typography, control feedback and screenshot critique; do not add stock photos, promotional hero copy, required motion counts or random marketing layouts to financial controls. Retain existing fonts/theme tokens. No invented payment data.
- Confirmed: mobile-only redesign and standalone history; unpaid orders can be cancelled, late receipts require review. PC/agent receive only cancelled-state compatibility.

- Final route design reuses DepositDetailView as a shell for distinct payment, history and historical-detail routes, avoiding another route chunk. Shared login styling is confined to deposit selectors in mobile/src/styles/deposit.css.
- Screenshot critique: make payment QR/address a single column even inside a desktop-hosted phone canvas; give the body-teleported cancellation sheet the Pencil theme tokens. Both corrections were made and screenshots re-inspected.
- Deduplication reduced the initial PWA output from 1516.4/661.3 KiB JS/CSS and 52 CSS chunks to 1514.2/658.1 KiB and 50 chunks. Feature budgets are narrowly adjusted to 1516/659 KiB raw; entry, gzip, counts and source-size budgets stay unchanged.
