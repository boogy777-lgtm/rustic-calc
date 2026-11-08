//! Neutral reconstructed skins. No original artwork is embedded here.

use super::color::{rgb, Color};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    System,
    White,
    Dark,
}

impl ThemeMode {
    pub fn resolve(self, system_dark: bool) -> &'static Palette {
        match self {
            Self::System => {
                if system_dark {
                    &DARK
                } else {
                    &LIGHT
                }
            }
            Self::White => &WHITE,
            Self::Dark => &DARK,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg_top: Color,
    pub bg_bottom: Color,
    pub panel: Color,
    pub panel_border: Color,
    pub display_bg: Color,
    pub key_top: Color,
    pub key_bottom: Color,
    pub key_hover: Color,
    pub key_pressed: Color,
    pub selection_bg: Color,
    pub selection_text: Color,
    pub op_top: Color,
    pub op_bottom: Color,
    pub eq_top: Color,
    pub eq_bottom: Color,
    pub eq_border: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub error: Color,
    pub control_bg: Color,
    pub control_text: Color,
    pub control_hover_border: Color,
    pub control_focus_border: Color,
    pub bit_on: Color,
    pub bit_off: Color,
    pub disabled_bg: Color,
    pub disabled_border: Color,
    pub disabled_text: Color,
}

pub const LIGHT: Palette = Palette {
    bg_top: rgb(222, 236, 249),
    bg_bottom: rgb(185, 208, 230),
    panel: rgb(235, 244, 251),
    panel_border: rgb(168, 189, 208),
    display_bg: rgb(249, 252, 255),
    key_top: rgb(247, 250, 253),
    key_bottom: rgb(235, 242, 248),
    key_hover: rgb(253, 244, 219),
    key_pressed: rgb(215, 232, 247),
    selection_bg: rgb(211, 227, 243),
    selection_text: rgb(30, 40, 50),
    op_top: rgb(246, 250, 254),
    op_bottom: rgb(232, 241, 249),
    eq_top: rgb(255, 226, 168),
    eq_bottom: rgb(245, 184, 76),
    eq_border: rgb(214, 154, 54),
    border: rgb(190, 205, 220),
    text: rgb(30, 40, 50),
    muted: rgb(90, 105, 120),
    error: rgb(164, 50, 42),
    control_bg: rgb(255, 255, 255),
    control_text: rgb(30, 40, 50),
    control_hover_border: rgb(132, 163, 194),
    control_focus_border: rgb(67, 113, 160),
    bit_on: rgb(28, 86, 140),
    bit_off: rgb(120, 140, 158),
    disabled_bg: rgb(224, 232, 241),
    disabled_border: rgb(198, 209, 221),
    disabled_text: rgb(145, 157, 171),
};

pub const WHITE: Palette = Palette {
    bg_top: rgb(255, 255, 255),
    bg_bottom: rgb(238, 242, 247),
    panel: rgb(249, 251, 253),
    panel_border: rgb(201, 210, 220),
    display_bg: rgb(255, 255, 255),
    key_top: rgb(255, 255, 255),
    key_bottom: rgb(240, 243, 247),
    key_hover: rgb(232, 240, 251),
    key_pressed: rgb(216, 230, 247),
    selection_bg: rgb(218, 232, 246),
    selection_text: rgb(30, 40, 50),
    op_top: rgb(252, 253, 255),
    op_bottom: rgb(238, 242, 247),
    eq_top: rgb(255, 226, 168),
    eq_bottom: rgb(245, 184, 76),
    eq_border: rgb(214, 154, 54),
    border: rgb(194, 204, 216),
    text: rgb(30, 40, 50),
    muted: rgb(90, 105, 120),
    error: rgb(164, 50, 42),
    control_bg: rgb(255, 255, 255),
    control_text: rgb(30, 40, 50),
    control_hover_border: rgb(140, 164, 191),
    control_focus_border: rgb(76, 124, 176),
    bit_on: rgb(28, 86, 140),
    bit_off: rgb(120, 140, 158),
    disabled_bg: rgb(236, 239, 243),
    disabled_border: rgb(210, 216, 223),
    disabled_text: rgb(160, 168, 178),
};

pub const DARK: Palette = Palette {
    bg_top: rgb(30, 36, 48),
    bg_bottom: rgb(18, 23, 34),
    panel: rgb(35, 43, 56),
    panel_border: rgb(58, 70, 88),
    display_bg: rgb(15, 20, 28),
    key_top: rgb(58, 69, 86),
    key_bottom: rgb(43, 52, 66),
    key_hover: rgb(74, 86, 106),
    key_pressed: rgb(56, 70, 90),
    selection_bg: rgb(62, 77, 99),
    selection_text: rgb(238, 243, 250),
    op_top: rgb(62, 74, 92),
    op_bottom: rgb(48, 58, 73),
    eq_top: rgb(122, 90, 30),
    eq_bottom: rgb(144, 96, 16),
    eq_border: rgb(112, 74, 10),
    border: rgb(70, 83, 106),
    text: rgb(232, 237, 245),
    muted: rgb(154, 167, 186),
    error: rgb(255, 123, 107),
    control_bg: rgb(26, 32, 41),
    control_text: rgb(232, 237, 245),
    control_hover_border: rgb(99, 121, 150),
    control_focus_border: rgb(137, 172, 212),
    bit_on: rgb(90, 169, 255),
    bit_off: rgb(126, 142, 165),
    disabled_bg: rgb(26, 32, 41),
    disabled_border: rgb(52, 62, 78),
    disabled_text: rgb(105, 116, 133),
};
