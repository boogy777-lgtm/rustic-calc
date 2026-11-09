# Changelog

## 0.4.0 — 2026-11-09

- Eleventh mode: offline WGS84 → MSK, region/system/zone selection, Ctrl+G,
  and `--mode coordinates` (`wgs84`/`msk` aliases).
- Editable PROJ templates, custom definitions, per-template/session drafts,
  strict validation, explicit ellipsoidal height and selectable X/Y results.
- 262 Apache-2.0 community definitions covering 79 regions. All 89 regional
  groups are selectable; missing definitions require manual input, not invented
  parameters. Source revision, limitations and full license are embedded in F2 help.
- Independent proj4.js numerical fixtures for every template. These are not
  surveying/control-point certification; no legal/cadastral accuracy is promised.

The application remains MIT, with no additional runtime Rust dependencies.

## 0.3.0 — 2026-11-08

First public Rustic Calc release, for Windows x64.

- Native Win32/GDI UI, ten modes, RU/EN and three themes.
- Correct inverse financial/fuel calculations and strict date input handling.
- Consistent field/drop-down frames and native editing behavior.
- Shared DPI multiplier, complete fractional-DPI key grids, monitor-aware menus.
- Reversible per-user CMD integration, portable EXE and per-user installer.
- Contributor/LLM module contracts, regression tests and release automation.

Linux, macOS and additional native Windows architectures are not released yet;
contributions are welcome.
