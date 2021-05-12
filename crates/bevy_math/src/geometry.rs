use bevy_reflect::Reflect;
use glam::Vec2;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

/// A two dimensional "size" as defined by a width and height
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct Size<T: Reflect + PartialEq = f32> {
    pub width: T,
    pub height: T,
}

impl<T: Reflect + PartialEq> Size<T> {
    pub fn new(width: T, height: T) -> Self {
        Size { width, height }
    }
}

impl<T: Default + Reflect + PartialEq> Default for Size<T> {
    fn default() -> Self {
        Self {
            width: Default::default(),
            height: Default::default(),
        }
    }
}

/// A rect, as defined by its "side" locations
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct Rect<T: Reflect + PartialEq> {
    pub left: T,
    pub right: T,
    pub top: T,
    pub bottom: T,
}

impl<T: Reflect + PartialEq> Rect<T> {
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

impl<T: Default + Reflect + PartialEq> Default for Rect<T> {
    fn default() -> Self {
        Self {
            left: Default::default(),
            right: Default::default(),
            top: Default::default(),
            bottom: Default::default(),
        }
    }
}

impl<T: Reflect + PartialEq> Add<Vec2> for Size<T>
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

impl<T: Reflect + PartialEq> AddAssign<Vec2> for Size<T>
where
    T: AddAssign<f32>,
{
    fn add_assign(&mut self, rhs: Vec2) {
        self.width += rhs.x;
        self.height += rhs.y;
    }
}

impl<T: Reflect + PartialEq> Sub<Vec2> for Size<T>
where
    T: Sub<f32, Output = T>,
{
    type Output = Size<T>;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Self {
            width: self.width - rhs.x,
            height: self.height - rhs.y,
        }
    }
}

impl<T: Reflect + PartialEq> SubAssign<Vec2> for Size<T>
where
    T: SubAssign<f32>,
{
    fn sub_assign(&mut self, rhs: Vec2) {
        self.width -= rhs.x;
        self.height -= rhs.y;
    }
}

impl<T: Reflect + PartialEq> Mul<f32> for Size<T>
where
    T: Mul<f32, Output = T>,
{
    type Output = Size<T>;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::Output {
            width: self.width * rhs,
            height: self.height * rhs,
        }
    }
}

impl<T: Reflect + PartialEq> MulAssign<f32> for Size<T>
where
    T: MulAssign<f32>,
{
    fn mul_assign(&mut self, rhs: f32) {
        self.width *= rhs;
        self.height *= rhs;
    }
}

impl<T: Reflect + PartialEq> Div<f32> for Size<T>
where
    T: Div<f32, Output = T>,
{
    type Output = Size<T>;

    fn div(self, rhs: f32) -> Self::Output {
        Self::Output {
            width: self.width / rhs,
            height: self.height / rhs,
        }
    }
}

impl<T: Reflect + PartialEq> DivAssign<f32> for Size<T>
where
    T: DivAssign<f32>,
{
    fn div_assign(&mut self, rhs: f32) {
        self.width /= rhs;
        self.height /= rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_ops() {
        type SizeF = Size<f32>;

        assert_eq!(
            SizeF::new(10., 10.) + Vec2::new(10., 10.),
            SizeF::new(20., 20.)
        );
        assert_eq!(
            SizeF::new(20., 20.) - Vec2::new(10., 10.),
            SizeF::new(10., 10.)
        );
        assert_eq!(SizeF::new(10., 10.) * 2., SizeF::new(20., 20.));
        assert_eq!(SizeF::new(20., 20.) / 2., SizeF::new(10., 10.));

        let mut size = SizeF::new(10., 10.);

        size += Vec2::new(10., 10.);

        assert_eq!(size, SizeF::new(20., 20.));
    }
}
