/// GDI `COLORREF` layout: `0x00BBGGRR`.
pub type Color = u32;

pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub const fn red(color: Color) -> u8 {
    (color & 0xff) as u8
}

pub const fn green(color: Color) -> u8 {
    ((color >> 8) & 0xff) as u8
}

pub const fn blue(color: Color) -> u8 {
    ((color >> 16) & 0xff) as u8
}
