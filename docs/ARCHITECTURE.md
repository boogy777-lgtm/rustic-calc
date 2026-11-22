# Architecture and dependency direction

```text
Win32: main -> app -> views -> rustic-calc-app
                     |   ui -> platform-safe resources
                     +------> platform (Win32 boundary)
Slint / GPUI: frontend adapter -> rustic-calc-app
```

This is a source-level application architecture, not a stable binary plugin ABI.

| Layer | Responsibility | Must not own |
| --- | --- | --- |
| engine | Arithmetic, scientific/programmer/statistics state | HWNDs, I/O, theme, localization policy |
| units | Unit/date/worksheet calculations and typed IDs | Control layout or message handling |
| coordinates | Strict CRS parsing, WGS84/ECEF/datum/TM math, embedded template data | HWNDs, I/O, network, localization policy |
| `rustic-calc-app::date` | Raw date drafts, typed operations, validation and results | Editor state, translated strings, system clock |
| `rustic-calc-app::keypad` | Semantic calculator commands, arithmetic/programmer state and statistical operations | Native key codes, controls, view lifecycle |
| `rustic-calc-app::{unit_session,worksheet,coordinate_session}` | Editable drafts, typed choices, validation and numerical results | Window handles or toolkit-specific models |
| `rustic-calc-app::startup` / `i18n` | CLI parsing, locale and theme choices, RU/EN strings | Opening windows, selecting a native palette, writing screenshots |
| ui | Canvas, reusable widgets, DPI, metrics, palette | Financial formulas or mode dispatch |
| platform | Native resources, controls, menus, clipboard, screenshots | Domain formulas or view-specific values |
| views | Local view model, field synchronization, actions and layout | New unsafe FFI or registry/network integration |
| app | Active mode, theme/locale, global commands and lifecycle | A second copy of the math engine |

The eleventh view is `views/coordinates.rs`; numerical code is isolated in
`crates/calc-app/src/coordinates/`. The Apache-2.0 catalogue is embedded with its full license, not
downloaded. Visible text/region names are in `i18n/coordinates.rs`. Drafts are
per-template/per-region session state, not filesystem persistence. See
[coordinate contract and provenance](COORDINATES.md).

The Win32 adapter obtains the local date from `platform::local_date()` when it
creates a date session. Tests and other frontends can supply a date directly.
The shared crate is dependency-free; its session exposes raw text and typed
results so a frontend never needs to create another frontend's controls.
The Win32 adapters use the shared calculator state without exporting
`ViewManager`, `Shared`, or the HWND-based control lifecycle. Each frontend
owns widget identity, event delivery, focus and DPI. Different processes have
independent sessions; sharing the library does not synchronize their state.

Slint and GPUI keep their manifests and lockfiles separate so their dependency
graphs cannot alter a normal Win32 build or the dependency-free application
crate. See [local edition packaging](RELEASING.md#local-three-edition-bundle).
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
