//! Reusable native GDI toolkit.
//!
//! Layers:
//! - [`color`] / [`palette`] — colors and the neutral reconstructed skin;
//! - [`metrics`] — logical size, spacing and font tokens;
//! - [`geometry`] — rectangles and the [`Stack`] layout cursor;
//! - [`canvas`] — double-buffered GDI primitives;
//! - [`widgets`] — semantic controls used by views.

mod canvas;
pub mod color;
pub mod dpi;
mod font;
mod geometry;
mod hit;
pub mod metrics;
pub mod palette;
mod widgets;

pub use canvas::Canvas;
pub use dpi::DpiScale;
pub use font::Fonts;
pub use geometry::{Rect, Stack, TextAlign};
pub use hit::Hit;
#[cfg(test)]
pub use widgets::key_cells;
pub use widgets::Ui;
