use std::ops::{Add, AddAssign, Mul, MulAssign};

use crate::{
    impl_render_resource_bytes,
    renderer::{RenderResource, RenderResourceType},
    texture::Texture,
};
use bevy_asset::Handle;
use bevy_core::{Byteable, Bytes};
use bevy_math::{Vec3, Vec4};
use bevy_reflect::{Reflect, ReflectDeserialize};
use serde::{Deserialize, Serialize};

/// HSLA Color
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct Hsla {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

unsafe impl Byteable for Hsla {}

impl Hsla {
    /// New [`Color`] with alpha equal to 1.0
    pub const fn new(h: f32, s: f32, l: f32) -> Hsla {
        Hsla::with_alpha(h, s, l, 1.0)
    }

    /// New [`Color`]
    pub const fn with_alpha(h: f32, s: f32, l: f32, a: f32) -> Hsla {
        Hsla { h, s, l, a }
    }
}

impl Default for Hsla {
    fn default() -> Self {
        Hsla::new(0.0, 0.0, 1.0)
    }
}

impl AddAssign<Hsla> for Hsla {
    fn add_assign(&mut self, rhs: Hsla) {
        *self = Hsla {
            h: self.h + rhs.h,
            s: self.s + rhs.s,
            l: self.l + rhs.l,
            a: self.a + rhs.a,
        }
    }
}

impl Add<Hsla> for Hsla {
    type Output = Hsla;

    fn add(mut self, rhs: Hsla) -> Self::Output {
        self += rhs;
        self
    }
}

impl Mul<f32> for Hsla {
    type Output = Hsla;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<f32> for Hsla {
    fn mul_assign(&mut self, rhs: f32) {
        self.h *= rhs;
        self.s *= rhs;
        self.l *= rhs;
        //self.a *= rhs;
    }
}

impl Mul<Hsla> for Hsla {
    type Output = Hsla;

    fn mul(mut self, rhs: Hsla) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<Hsla> for Hsla {
    fn mul_assign(&mut self, rhs: Hsla) {
        self.h *= rhs.h;
        self.s *= rhs.s;
        self.l *= rhs.l;
        self.a *= rhs.a;
    }
}

impl From<Hsla> for [f32; 4] {
    fn from(color: Hsla) -> Self {
        [color.h, color.s, color.l, color.a]
    }
}

impl From<[f32; 3]> for Hsla {
    fn from([h, s, l]: [f32; 3]) -> Self {
        Hsla::with_alpha(h, s, l, 1.0)
    }
}

impl From<[f32; 4]> for Hsla {
    fn from([h, s, l, a]: [f32; 4]) -> Self {
        Hsla::with_alpha(h, s, l, a)
    }
}

impl From<Hsla> for Vec4 {
    fn from(color: Hsla) -> Self {
        Vec4::new(color.h, color.s, color.l, color.a)
    }
}

impl From<Vec3> for Hsla {
    fn from(vec4: Vec3) -> Self {
        Hsla::with_alpha(vec4.x, vec4.y, vec4.z, 1.0)
    }
}

impl From<Vec4> for Hsla {
    fn from(vec4: Vec4) -> Self {
        Hsla::with_alpha(vec4.x, vec4.y, vec4.z, vec4.w)
    }
}

impl_render_resource_bytes!(Hsla);

#[test]
fn test_conversions_vec4() {
    let starting_vec4 = Vec4::new(0.4, 0.5, 0.6, 1.0);
    let starting_color = Hsla::from(starting_vec4);

    assert_eq!(starting_vec4, Vec4::from(starting_color),);
}

#[test]
fn test_mul_and_mulassign_f32() {
    let starting_color = Hsla::with_alpha(0.4, 0.5, 0.6, 1.0);
    assert_eq!(
        starting_color * 0.5,
        Hsla::with_alpha(0.4 * 0.5, 0.5 * 0.5, 0.6 * 0.5, 1.0),
    );
}
