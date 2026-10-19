# Diagnosis

- Update errors previously displayed `registrationFailed` for every cause.
- The old updater accepted any controllerchange, renewed 15-second timeouts for each phase, and failed forever with `no-waiting-worker` after activation in another tab or a late activation following timeout.
- The shared modal watcher is not immediate; an `onMounted` visibility gate ensures initial updates receive focus and scroll locking. Installation and update share one modal, avoiding competing locks during priority changes.
- Public origin checked in Ego Browser on 2026-10-20: secure context, active service worker, no waiting worker. All 125 worker/precache resources checked via HEAD returned acceptable status/type. This does not reproduce the user's earlier failure and does not prove its exact historical cause.
- Actual `sw.js` response has `Cache-Control: max-age=14400`. Existing `mobile/README.md` release requirements prohibit strong CDN caching for HTML/worker/manifest. Deployment should bypass CDN caching for these entries and invalidate previously cached entries; no server/CDN settings were changed in this task.
- Initial shell HTTP inspection returned 403; browser access succeeded. No production storage was cleared and no business requests were written.
