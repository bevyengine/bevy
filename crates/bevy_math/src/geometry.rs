use std::ops::{Add, AddAssign};
use glam::Vec2;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

impl<T> Size<T> {
    pub fn new(width: T, height: T) -> Self {
        Size { width, height }
    }
}

impl<T: Default> Default for Size<T> {
    fn default() -> Self {
        Self {
            width: Default::default(),
            height: Default::default(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Rect<T> {
    pub start: T,
    pub end: T,
    pub top: T,
    pub bottom: T,
}

impl<T: Default> Default for Rect<T> {
    fn default() -> Self {
        Self {
            start: Default::default(),
            end: Default::default(),
            top: Default::default(),
            bottom: Default::default(),
        }
    }
}

impl<T> Add<Vec2> for Size<T> where T: Add<f32, Output=T> {
    type Output = Size<T>;
    fn add(self, rhs: Vec2) -> Self::Output {
        Self {
            width: self.width + rhs.x(),
            height: self.height + rhs.y(),
        }
    }
}

impl<T> AddAssign<Vec2> for Size<T> where T: AddAssign<f32> {
    fn add_assign(&mut self, rhs: Vec2) {
        self.width += rhs.x();
        self.height += rhs.y();
    }
}