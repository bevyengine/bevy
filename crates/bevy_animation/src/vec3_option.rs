use bevy_math::Vec3;
use std::ops::Add;

pub struct Vec3Option {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
}

impl Vec3Option {
    pub fn new(x: Option<f32>, y: Option<f32>, z: Option<f32>) -> Self {
        Self { x, y, z }
    }

    pub fn alter(&self, vec: &mut Vec3) {
        if let Some(x) = self.x {
            vec.set_x(x);
        }
        if let Some(y) = self.y {
            vec.set_y(y);
        }
        if let Some(z) = self.z {
            vec.set_z(z);
        }
    }

    pub fn zero() -> Self {
        Self {
            x: Some(0.0),
            y: Some(0.0),
            z: Some(0.0),
        }
    }

    pub fn none() -> Self {
        Self {
            x: None,
            y: None,
            z: None,
        }
    }
}

fn add_one(l: Option<f32>, r: Option<f32>) -> Option<f32> {
    if let (Some(l), Some(r)) = (l, r) {
        Some(l + r)
    } else {
        l.or(r)
    }
}

impl Add<Vec3Option> for Vec3Option {
    type Output = Vec3Option;

    fn add(self, other: Self) -> Self::Output {
        Self {
            x: add_one(self.x, other.x),
            y: add_one(self.y, other.y),
            z: add_one(self.z, other.z),
        }
    }
}

impl Add<Vec3Option> for Vec3 {
    type Output = Vec3;

    fn add(self, other: Vec3Option) -> Self::Output {
        let mut res = Vec3::zero();
        other.alter(&mut res);
        self + res
    }
}

impl Add<Vec3> for Vec3Option {
    type Output = Vec3;

    fn add(self, other: Vec3) -> Self::Output {
        let mut res = Vec3::zero();
        self.alter(&mut res);
        res + other
    }
}
