//! Logical pixel tokens shared by every view.
//!
//! Values are scaled by the current DPI in [`crate::ui::Ui::s`]; keeping them
//! here prevents the same magic number from drifting between modules.

// Fonts ---------------------------------------------------------------------
pub const FONT_FAMILY: &str = "Segoe UI";
pub const FONT_SMALL: i32 = 11;
pub const FONT_BODY: i32 = 13;
pub const FONT_LABEL: i32 = 12;
pub const FONT_BUTTON: i32 = 17;
pub const FONT_BUTTON_SMALL: i32 = 12;
pub const FONT_TITLE: i32 = 15;
pub const FONT_DISPLAY: i32 = 38;
pub const FONT_DISPLAY_MID: i32 = 28;
pub const FONT_DISPLAY_SMALL: i32 = 20;
pub const BUTTON_SMALL_LABEL: usize = 4;

// Spacing -------------------------------------------------------------------
pub const MARGIN: f32 = 10.0;
pub const GAP: f32 = 6.0;
pub const GAP_TIGHT: f32 = 4.0;
pub const GAP_HAIRLINE: f32 = 2.0;
pub const GAP_FORM: f32 = 8.0;
pub const GAP_BLOCK: f32 = 10.0;
pub const GAP_SECTION: f32 = 12.0;
pub const GAP_KEYPAD: f32 = 8.0;
/// Equal clear space above and below menu-bar buttons, excluding the 1 px separator.
pub const MENU_FOOT_GAP: f32 = 4.0;
pub const MENU_EDGE_PAD: f32 = 4.0;
pub const MENU_ITEM_GAP: f32 = 4.0;
pub const MENU_TEXT_PAD_X: f32 = 12.0;
pub const MENU_TEXT_PAD_Y: f32 = 3.0;
pub const POPUP_EDGE_PAD: f32 = 4.0;
pub const POPUP_ROW_H: f32 = 27.0;
pub const POPUP_SEPARATOR_H: f32 = 9.0;
pub const POPUP_CHECK_W: f32 = 26.0;
pub const POPUP_ARROW_W: f32 = 20.0;
pub const POPUP_MIN_W: f32 = 120.0;
/// Shared choice-control geometry for angles, numeric base and word size.
pub const SELECTOR_ITEM_H: f32 = 24.0;
pub const SELECTOR_GAP: f32 = 2.0;

// Radii ---------------------------------------------------------------------
pub const RADIUS_PANEL: f32 = 5.0;
pub const RADIUS_CONTROL: f32 = 4.0;
pub const RADIUS_ITEM: f32 = 3.0;
pub const RADIUS_GLOSS: f32 = 2.0;

// Paddings ------------------------------------------------------------------
pub const PAD_PANEL: f32 = 8.0;
pub const PAD_DISPLAY: f32 = 8.0;
pub const PAD_RESULT: f32 = 7.0;
pub const PAD_BUTTON: f32 = 3.0;
pub const GLOSS_H: f32 = 3.0;

// Display / history ---------------------------------------------------------
pub const DISPLAY_H: f32 = 74.0;
pub const EXPRESSION_H: f32 = 18.0;
pub const HISTORY_TITLE_H: f32 = 18.0;
pub const HISTORY_LINE_H: f32 = 15.0;
pub const HISTORY_LINE_STEP: f32 = 16.0;
pub const HISTORY_TOP: f32 = 30.0;
pub const HISTORY_BOTTOM: f32 = 22.0;
pub const HISTORY_FOOTER_H: f32 = 16.0;
pub const HISTORY_VISIBLE: usize = 9;

// Calculator panels ---------------------------------------------------------
pub const KEY_CELL_W: f32 = 51.0;
pub const KEY_CELL_H: f32 = 41.0;
pub const KEYPAD_COLS: usize = 5;
pub const KEYPAD_ROWS: usize = 6;
pub const KEYPAD_COLS_WIDE: usize = 8;
pub const KEY_GAP: f32 = GAP_KEYPAD;
pub const KEYPAD_W: f32 = KEYPAD_COLS as f32 * KEY_CELL_W + (KEYPAD_COLS - 1) as f32 * KEY_GAP;
pub const KEYPAD_WIDE_W: f32 =
    KEYPAD_COLS_WIDE as f32 * KEY_CELL_W + (KEYPAD_COLS_WIDE - 1) as f32 * KEY_GAP;
pub const KEYPAD_H: f32 = KEYPAD_ROWS as f32 * KEY_CELL_H + (KEYPAD_ROWS - 1) as f32 * KEY_GAP;
pub const SIDEBAR_W: f32 = KEYPAD_W;
pub const SIDEBAR_DISPLAY_H: f32 = 48.0;
pub const SIDEBAR_HISTORY_MIN_H: f32 = 126.0;
pub const COLUMN_GAP: f32 = GAP_BLOCK;
pub const SCIENTIFIC_ROWS: usize = 5;
pub const SCIENTIFIC_GRID_H: f32 =
    SCIENTIFIC_ROWS as f32 * KEY_CELL_H + (SCIENTIFIC_ROWS - 1) as f32 * KEY_GAP;
pub const PROGRAMMER_BASE_W: f32 = 180.0;
pub const PROGRAMMER_BITS_H: f32 = 62.0;
pub const COMPACT_CONTENT_H: f32 = MARGIN * 2.0 + DISPLAY_H + GAP + KEYPAD_H;
pub const PROGRAMMER_CONTENT_H: f32 = COMPACT_CONTENT_H + GAP + PROGRAMMER_BITS_H;
pub const FORM_CONTENT_H: f32 =
    MARGIN * 2.0 + SIDEBAR_HISTORY_MIN_H + SIDEBAR_DISPLAY_H + GAP * 2.0 + KEYPAD_H;

// Forms ---------------------------------------------------------------------
pub const FIELD_H: f32 = 26.0;
pub const FIELD_LABEL_H: f32 = 18.0;
pub const RESULT_H: f32 = 41.0;
pub const RESULT_SMALL_H: f32 = 41.0;
pub const FORM_COL_W: f32 = 190.0;
pub const FORM_COL_GAP: f32 = 30.0;
pub const FORM_TITLE_H: f32 = 24.0;
pub const FORM_ACTION_H: f32 = 41.0;
pub const FORM_ACTION_W: f32 = 124.0;
pub const COMBO_POPUP_H: f32 = 206.0;
pub const COMBO_LIST_H: f32 = COMBO_POPUP_H - FIELD_H;
pub const COMBO_ITEM_H: f32 = SELECTOR_ITEM_H;
pub const COMBO_ARROW_MIN_W: f32 = 12.0;
pub const COMBO_ARROW_MIN_HALF: f32 = 3.0;
pub const COMBO_ARROW_MIN_H: f32 = 2.0;
pub const SCROLL_THUMB_PAD: f32 = 3.0;

// Coordinate form (no calculator sidebar) ------------------------------------
pub const COORDINATE_CONTENT_W: f32 = 900.0;
pub const COORDINATE_EDITOR_H: f32 = 100.0;
pub const COORDINATE_TOOL_W: f32 = 190.0;
pub const COORDINATE_CONTENT_H: f32 = MARGIN * 2.0
    + FORM_TITLE_H
    + GAP_SECTION
    + FIELD_LABEL_H
    + GAP_HAIRLINE
    + FIELD_H
    + GAP_SECTION
    + FIELD_H
    + GAP_HAIRLINE
    + COORDINATE_EDITOR_H
    + GAP_HAIRLINE
    + FIELD_LABEL_H
    + GAP_SECTION
    + FIELD_LABEL_H
    + GAP_HAIRLINE
    + FIELD_H
    + GAP_SECTION
    + FORM_ACTION_H
    + GAP_TIGHT
    + FIELD_LABEL_H
    + GAP_SECTION
    + FIELD_LABEL_H
    + GAP_HAIRLINE
    + FIELD_H
    + GAP_SECTION
    + FIELD_LABEL_H * 2.0
    + GAP_HAIRLINE
    + FIELD_H;
