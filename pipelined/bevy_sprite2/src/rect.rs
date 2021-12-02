use bevy_math::Vec2;

/// A rectangle defined by two points. There is no defined origin, so 0,0 could be anywhere
/// (top-left, bottom-left, etc)
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct Rect {
    /// The beginning point of the rect
    pub min: Vec2,
    /// The ending point of the rect
    pub max: Vec2,
}

impl Rect {
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn size(&self) -> Vec2 {
        Vec2::new(self.width(), self.height())
    }
}
