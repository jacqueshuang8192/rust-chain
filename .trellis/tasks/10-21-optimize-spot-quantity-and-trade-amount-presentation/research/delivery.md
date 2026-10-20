# Delivery

- Mobile spot quantity now displays up to8 decimal places and turnover follows quote-asset precision (USDT2); existing financial formatter provides grouping and tiny-nonzero thresholds. Exact monetary state and submission remain unchanged.
- Focus restores raw input, blur changes display only. Quote draft preserves user typing instead of immediately multiplying the divided quantity back into the field. Manual edits clear percentage selection.
- Actual SFC computed code behavior tests cover screenshot quantity0.540055112180188796 ->0.54005511, turnover44900.85169499999993354704 ->44,900.85, focus/blur exact equality,100.50 typing, tiny and unsafe-integer values.
- Final npm --prefix mobile run release:gate passed894/894, production/test types, PWA/Tauri Web build/artifacts/budgets/governance. Log /tmp/spot-input-release.log. PWA JS1491.8KiB, Tauri1488.2KiB; budgets unchanged.
- Existing Pencil digest initially failed on the two requested input changes. Added exact normalization for those two replacements, preserving the original digest for the rest of the spot template; full gate then passed.
- Ego actual app isolated numeric fixture verified formatted fields, focus restoration, stable100.50 quote draft, no page overflow and zero writes. Screenshot /tmp/spot-input-final.png inspected; screenshot showed desktop shell, so no narrow-screen visual claim. Space2 closed and own5187 preview stopped.
- No backend, PC, contract, production API or native-device changes/verification. Local commit only; not pushed or deployed.
