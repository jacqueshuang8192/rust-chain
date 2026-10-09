# Make Mobile Default Theme Dark

## Goal

Make the mobile H5/Tauri shell start in the dark theme by default while
preserving an explicit user-selected light or dark theme.

## Requirements

- An absent, invalid or unsupported stored theme resolves to `dark`.
- A valid stored `light` value remains `light`; a valid stored `dark` value
  remains `dark`.
- Initial HTML and PWA manifest metadata use the dark surface so the first
  paint and install shell match the application default.
- Remove the system color-scheme preference from default selection.
- Keep the existing theme toggle, persistence key, runtime meta-color updates,
  light theme tokens and dark theme tokens unchanged.

## Acceptance Criteria

- [x] Theme resolver defaults to `dark` for absent/invalid values.
- [x] Explicit stored light theme remains supported.
- [x] Initial HTML carries the dark theme and dark theme-color.
- [x] PWA manifest defaults to dark theme/background colors.
- [x] Theme regression tests and the mobile release gate pass.

## Out of Scope

- No redesign of light/dark color tokens.
- No change to the theme toggle behavior or storage key.
- No PC theme changes.

## Verification

- Run focused theme tests, then `npm run release:gate` from `mobile/`.
- Run `git diff --check` and Trellis task validation.

## Delivery

- Theme regression coverage and the mobile release gate both pass.
- PWA manifest was rebuilt and confirmed with `theme_color` and
  `background_color` set to `#070908`.
- Work remains uncommitted pending the user's commit/push instruction.
