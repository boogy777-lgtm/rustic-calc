# Module SDK: source contribution contract

There is **no dynamic plugin loader, DLL ABI or external plugin package format**.
"SDK" here means the supported source-level process for contributing modules.
Internal Rust interfaces may change between versions; do not promise ABI stability.

## New calculator/view checklist

1. Discuss the module's purpose, inputs, validation and supported targets in an
   issue. Dependency/platform changes require approval before implementation.
2. Put numerical/state logic in a focused core module. Use typed inputs, results
   and errors; reject missing, nonnumeric, nonfinite and out-of-range data.
   Do not silently turn invalid fields into zero or require the target as input.
3. Add a `View` implementation in `src/views/`. Implement `mode`, `paint`, `action`
   and necessary `command`, `on_enter`, `on_exit`, `on_resize` methods.
4. Register the mode explicitly: `Mode`, CLI parser, command IDs/menu, view factory,
   manager index/capacity and preferred content size. Existing ten-mode slots and
   shortcuts are not automatically extensible. Do not overwrite an existing slot
   or hotkey. For an eleventh mode, replace/update that registration coherently.
5. Use `Shared::edit`/`combo`, shared widgets and `Action`; do not invent native
   control subclasses or embed raw HWND calls in the view. New reusable behavior
   belongs in `platform`/`ui` and needs its own lifecycle tests.
6. Add RU/EN labels/errors/help in `i18n.rs`. Add stable IDs for business entities;
   never compare display labels to select functionality.
7. Sync native input on EN_CHANGE, invalidate only relevant result areas when
   practical, and retain uncalculated edits through mode/theme/DPI changes.
8. Provide default/minimum sizing derived from layout tokens and measured text;
   verify 75–400% DPI, span coverage, empty/error/dirty states and all solve targets.
9. Update menu/help/README/tests/architecture notes in the same PR.

## Required tests

- Pure calculations: normal, boundary, invalid and nonfinite inputs; inverse
  targets round-trip; repeat-equals/word masking where relevant.
- Layout: no overlaps/clipping; complete grid bounds; valid native field focus;
  RU/EN and light/dark captions with realistic longest text.
- Lifecycle: entry/exit, native ownership, DPI 96→144→192→96 without accumulated
  scale, and preservation of unsaved input.
- Keyboard: Tab/Shift+Tab/Enter/Esc, existing global shortcuts from native fields,
  and native editing commands. Do not use the user's clipboard in automated tests.
- Document whether evidence is a formula test, hidden-window test, synthetic
  messages or actual physical multi-monitor interaction.

## OS/backend contributions

Linux/macOS PRs are welcome. Separate a genuine backend from the numerical core;
do not ship Wine wrappers as native builds. Introduce target gating and backend
tests before changing the support table or release matrix. New runtime dependency,
network capability, filesystem access or plugin loading needs explicit review.

## Compatibility promises

Do not remove existing CLI options, actions, locales, themes or modes without a
documented breaking release. Do not change installer AppId, integration ownership
IDs or uninstall semantics casually: they identify existing user-owned state.
