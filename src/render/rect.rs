use crate::math::{Vec2, Vec4};

pub struct Anchors {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,

}

impl Anchors {
    pub fn new(top: f32, bottom: f32, left: f32, right: f32) -> Self {
        Anchors {
            top,
            bottom,
            left,
            right
        }
    }
}

impl Default for Anchors {
    fn default() -> Self {
        Anchors {
            top: 0.5,
            bottom: 0.5,
            left: 0.5,
            right: 0.5
        }
    }
}

pub struct Rect {
    pub position: Vec2,
    pub dimensions: Vec2,
    pub anchors: Anchors,
    pub color: Vec4,
}

impl Rect {
    pub fn new(position: Vec2, dimensions: Vec2, anchors: Anchors, color: Vec4) -> Self {
        Rect {
            position,
            dimensions,
            anchors,
            color,
        }
    }
}
