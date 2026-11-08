use windows_sys::Win32::Foundation::RECT;

/// Inclusive start/exclusive end for each cell; all rounding remainder is used.
pub(crate) fn axis_cells(start: i32, length: i32, count: usize, gap: i32) -> Vec<(i32, i32)> {
    if count == 0 {
        return Vec::new();
    }
    let count = count as i32;
    let available = length - gap * (count - 1);
    (0..count)
        .map(|i| {
            (
                start + available * i / count + gap * i,
                start + available * (i + 1) / count + gap * i,
            )
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    pub fn inset(&self, n: i32) -> Self {
        self.pad(n, n)
    }

    pub fn pad(&self, x: i32, y: i32) -> Self {
        Self::new(self.x + x, self.y + y, self.w - x * 2, self.h - y * 2)
    }

    pub fn columns(&self, count: usize, gap: i32) -> Vec<Self> {
        axis_cells(self.x, self.w, count, gap)
            .into_iter()
            .map(|(start, end)| Self::new(start, self.y, end - start, self.h))
            .collect()
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    pub fn to_win(&self) -> RECT {
        RECT {
            left: self.x,
            top: self.y,
            right: self.right(),
            bottom: self.bottom(),
        }
    }
}

/// A vertical layout cursor: views declare a size and the stack advances.
#[derive(Clone, Copy)]
pub struct Stack {
    area: Rect,
    gap: i32,
    cursor: i32,
}

impl Stack {
    pub fn new(area: Rect, gap: i32) -> Self {
        Self {
            area,
            gap,
            cursor: area.y,
        }
    }

    pub fn take(&mut self, height: i32) -> Rect {
        let rect = Rect::new(self.area.x, self.cursor, self.area.w, height);
        self.cursor += height + self.gap;
        rect
    }

    pub fn advance(&mut self, gap: i32) {
        self.cursor += gap;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl TextAlign {
    pub fn flags(self) -> u32 {
        match self {
            Self::Left => windows_sys::Win32::Graphics::Gdi::DT_LEFT,
            Self::Center => windows_sys::Win32::Graphics::Gdi::DT_CENTER,
            Self::Right => windows_sys::Win32::Graphics::Gdi::DT_RIGHT,
        }
    }
}
