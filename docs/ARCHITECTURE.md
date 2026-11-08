# Architecture and dependency direction

```text
main -> app -> views -> engine / units
             |        ui -> platform-safe resources
             +------> platform (Win32 boundary)
```

This is a source-level application architecture, not a stable binary plugin ABI.

| Layer | Responsibility | Must not own |
| --- | --- | --- |
| engine | Arithmetic, scientific/programmer/statistics state | HWNDs, I/O, theme, localization policy |
| units | Unit/date/worksheet calculations and typed IDs | Control layout or message handling |
| ui | Canvas, reusable widgets, DPI, metrics, palette | Financial formulas or mode dispatch |
| platform | Native resources, controls, menus, clipboard, screenshots | Domain formulas or view-specific values |
| views | Local view model, field synchronization, actions and layout | New unsafe FFI or registry/network integration |
| app | Active mode, theme/locale, global commands and lifecycle | A second copy of the math engine |

The existing `units::today()` uses one `platform::local_date()` bridge. Do not
extend this exception; a portability PR should inject the clock or move the bridge.
Win32 handle types/notification constants in views are permitted compatibility
types; unsafe API calls stay behind safe helpers.

## State and lifetime

Views are cached by `ViewManager`; native children are created on entry and
destroyed on exit/DPI rebuild. EN_CHANGE must synchronize text immediately so
uncalculated edits survive recreation. Clear/mark results when inputs change.
Never rebuild fields on every keystroke. Preserve native clipboard/caret/selection.

Popup styles borrow cached fonts: close popups before replacing fonts. Destroy
HWND/native state outside active mutable callback borrows. Follow existing
deferred close/select messages rather than freeing a callback's own state.

## Units and layout

Keep source sizes in logical DP. Use `DpiScale`, `Shared::s` or `Ui::s` once to
produce physical rectangles. Do not clamp a valid scale to 100%. Grid partitions
must consume all pixels and span keys must use shared edges. Native RECTs and
font/scrollbar metrics are already physical; never multiply them again.

Theme colors come from `Palette`; visible text from `i18n::Text`. Independent
local layout choices may be named constants, but derived sizes must be formulas
over shared tokens. Category/command identity must not depend on translated names.
