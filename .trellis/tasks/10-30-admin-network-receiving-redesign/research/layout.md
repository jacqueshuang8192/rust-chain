# Network receiving workbench

- User selected network workbench on 2026-10-30.
- Preserve existing Settings/Scanner/Asset/SharedAddress DTOs and pinned editor model.
- NetworkNavigation shares the backend networkOptions list; desktop buttons and narrow select use the same network state.
- NetworkOverview reuses methodBlockers/scanStatus; configured channel count is not health. A failed or pending read must hide ready claims.
- Three Semi Tabs mount only the selected panel. Existing editors remain outside panels so save/error/discard state is independent.
- Lock network/section changes while an editor or address confirmation is open; pause polling as before. Refresh must remain available after an error, so refreshing and unavailable are distinct.
- Keep AdminLayout and outer RechargeWorkspace navigation; full-width page style is scoped to deposit-workspace-page.
- Official Semi MCP Tabs documentation 2.99.2 verified keepDOM=false, disabled TabPane and accessible panel linkage.
- Browser fixture adds optional shell/addressonly/error modes using local simulated API responses. No production calls or secrets.
- Regression tests must select the desktop network buttons: the mobile combobox is intentionally display:none at desktop widths.
