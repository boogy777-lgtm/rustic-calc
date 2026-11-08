use crate::actions::Action;

use super::geometry::Rect;

#[derive(Clone, Copy)]
pub struct Hit {
    pub id: u64,
    pub rect: Rect,
    pub action: Action,
}

pub fn find(hits: &[Hit], x: i32, y: i32) -> Option<Hit> {
    hits.iter()
        .rev()
        .find(|hit| hit.rect.contains(x, y))
        .copied()
}
