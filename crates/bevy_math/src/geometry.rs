use bevy_reflect::Reflect;
use glam::Vec2;
use std::ops::{Add, AddAssign};

/// A two dimensional "size" as defined by a width and height
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
pub struct Size<T: Reflect = f32> {
    pub width: T,
    pub height: T,
}

impl<T: Reflect> Size<T> {
    pub fn new(width: T, height: T) -> Self {
        Size { width, height }
    }
}

impl<T: Default + Reflect> Default for Size<T> {
    fn default() -> Self {
        Self {
            width: Default::default(),
            height: Default::default(),
        }
    }
}

/// A rect, as defined by its "side" locations
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
pub struct Rect<T: Reflect> {
    pub left: T,
    pub right: T,
    pub top: T,
    pub bottom: T,
}

impl<T: Reflect> Rect<T> {
    pub fn all(value: T) -> Self
    where
        T: Clone,
    {
        Rect {
            left: value.clone(),
            right: value.clone(),
            top: value.clone(),
            bottom: value,
        }
    }
}

impl<T: Default + Reflect> Default for Rect<T> {
    fn default() -> Self {
        Self {
            left: Default::default(),
            right: Default::default(),
            top: Default::default(),
            bottom: Default::default(),
        }
    }
}

impl<T: Reflect> Add<Vec2> for Size<T>
where
    T: Add<f32, Output = T>,
{
    type Output = Size<T>;

    fn add(self, rhs: Vec2) -> Self::Output {
        Self {
            width: self.width + rhs.x,
            height: self.height + rhs.y,
        }
    }
}

impl<T: Reflect> AddAssign<Vec2> for Size<T>
where
    T: AddAssign<f32>,
{
    fn add_assign(&mut self, rhs: Vec2) {
        self.width += rhs.x;
        self.height += rhs.y;
    }
}
