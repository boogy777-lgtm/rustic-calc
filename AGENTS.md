# Working on Rustic Calc with an LLM

Read `README.md`, `docs/ARCHITECTURE.md`, `docs/MODULE_SDK.md` and
`docs/LLM_RULES.md` before changing code. These are project instructions, not an
OpenCode/Copilot-specific agent configuration.

## Non-negotiable constraints

- Windows x64 is the only released platform. Never label a Windows EXE as a
  Linux/macOS build. Porting PRs need a real backend and a test matrix.
- `windows-sys 0.59` is the only direct/runtime Rust dependency. A dependency,
  backend, plugin ABI or platform expansion requires maintainer approval.
- Preserve ten existing modes, RU default + EN, System/White/Dark themes,
  `Ctrl+1..0`, `Ctrl+U/E`, native field editing and CLI screenshot flags.
- DPI is `dp * dpi / 96`, once. Keep DP and physical PX distinguishable;
  native Windows measurements/suggested rectangles are already PX.
- No `target-cpu=native` in distributed builds. No update downloader,
  telemetry, network calls, shell execution or registry writes in calculators.
- Registry integration is confined to the opt-in per-user installer/command
  helper. Never replace System32 files, change HKLM or rewrite persistent PATH.
- No original executables, resource dumps, decompiled code, credentials,
  proprietary captures, private paths or local investigation ledgers in Git.
- Do not overwrite/revert another contributor's work. Do not push, publish,
  create tags, edit release assets or install software without explicit approval.

## Module boundaries

- `engine.rs`: pure numerical/state logic. No UI, Win32, filesystem or network.
- `units.rs`: conversion/date/worksheet logic; the existing `today()` bridge is
  the only current platform dependency. Do not add more without extracting it.
- `platform/`: native API boundary, HWND/message/control/resource lifetimes.
- `ui/`: reusable rendering, DPI, geometry, palettes and semantic controls.
- `views/`: composition and input→action/state wiring, through safe platform
  helpers. Do not introduce unsafe blocks or direct OS calls in production views.
- `app.rs`/`main.rs`: coordination/registration, not new domain algorithms.
- Visible strings belong in `i18n.rs`; shared geometry in `ui/metrics.rs`.

For a new module follow `docs/MODULE_SDK.md`; do not create an undocumented
parallel framework. There is no stable dynamic plugin SDK or DLL loading.

## Required evidence

```text
cargo fmt --check
python tools/check_project.py
cargo test --locked
cargo build --release --locked
```

For command/installer changes run the isolated PowerShell tests. For UI changes
capture affected modes in both themes/locales, test native focus/editing and keep
the DPI matrix 72/96/120/144/168/192/240/288/384. A hidden/synthetic test is not a
physical multi-monitor test: report the distinction. Do not invent test counts,
platform support, binary sizes, checksums or release success.

Use small patches; add regression tests with each fix. If a constraint cannot be
met, stop at that boundary and explain the blocker rather than bypassing it.
