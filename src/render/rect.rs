use crate::math::{Vec2, Vec4};

pub struct Rect {
    pub position: Vec2,
    pub dimensions: Vec2,
    pub color: Vec4,
}

impl Rect {
    pub fn new(position: Vec2, dimensions: Vec2, color: Vec4) -> Self {
        Rect {
            position,
            dimensions,
            color,
        }
    }
}