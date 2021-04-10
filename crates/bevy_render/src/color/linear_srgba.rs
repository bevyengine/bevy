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

/// RGBA color in the Linear sRGB colorspace (often colloquially referred to as "linear", "RGB", or
/// "linear RGB").
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct LinSrgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

unsafe impl Byteable for LinSrgba {}

impl LinSrgba {
    pub const ALICE_BLUE: LinSrgba = LinSrgba::new(0.8689, 0.9331, 1.0);
    pub const ANTIQUE_WHITE: LinSrgba = LinSrgba::new(0.9551, 0.8276, 0.674);
    pub const AQUAMARINE: LinSrgba = LinSrgba::new(0.2049, 1.0, 0.656);
    pub const AZURE: LinSrgba = LinSrgba::new(0.8689, 1.0, 1.0);
    pub const BEIGE: LinSrgba = LinSrgba::new(0.9114, 0.9114, 0.711);
    pub const BISQUE: LinSrgba = LinSrgba::new(1.0, 0.7678, 0.554);
    pub const BLACK: LinSrgba = LinSrgba::new(0.0, 0.0, 0.0);
    pub const BLUE: LinSrgba = LinSrgba::new(0.0, 0.0, 1.0);
    pub const CRIMSON: LinSrgba = LinSrgba::new(0.7106, 0.0072, 0.047);
    pub const CYAN: LinSrgba = LinSrgba::new(0.0, 1.0, 1.0);
    pub const DARK_GRAY: LinSrgba = LinSrgba::new(0.0509, 0.0509, 0.051);
    pub const DARK_GREEN: LinSrgba = LinSrgba::new(0.0, 0.2140, 0.0);
    pub const FUCHSIA: LinSrgba = LinSrgba::new(1.0, 0.0, 1.0);
    pub const GOLD: LinSrgba = LinSrgba::new(1.0, 0.6739, 0.0);
    pub const GRAY: LinSrgba = LinSrgba::new(0.2140, 0.2140, 0.214);
    pub const GREEN: LinSrgba = LinSrgba::new(0.0, 1.0, 0.0);
    pub const INDIGO: LinSrgba = LinSrgba::new(0.0684, 0.0, 0.223);
    pub const LIME_GREEN: LinSrgba = LinSrgba::new(0.0331, 0.6038, 0.033);
    pub const MAROON: LinSrgba = LinSrgba::new(0.2140, 0.0, 0.0);
    pub const MIDNIGHT_BLUE: LinSrgba = LinSrgba::new(0.0100, 0.0100, 0.163);
    pub const NAVY: LinSrgba = LinSrgba::new(0.0, 0.0, 0.214);
    pub const NONE: LinSrgba = LinSrgba::with_alpha(0.0, 0.0, 0.0, 0.0);
    pub const OLIVE: LinSrgba = LinSrgba::new(0.2140, 0.2140, 0.0);
    pub const ORANGE: LinSrgba = LinSrgba::new(1.0, 0.3801, 0.0);
    pub const ORANGE_RED: LinSrgba = LinSrgba::new(1.0, 0.0593, 0.0);
    pub const PINK: LinSrgba = LinSrgba::new(1.0, 0.0072, 0.296);
    pub const PURPLE: LinSrgba = LinSrgba::new(0.2140, 0.0, 0.214);
    pub const RED: LinSrgba = LinSrgba::new(1.0, 0.0, 0.0);
    pub const SALMON: LinSrgba = LinSrgba::new(0.9551, 0.2140, 0.171);
    pub const SEA_GREEN: LinSrgba = LinSrgba::new(0.0272, 0.2633, 0.095);
    pub const SILVER: LinSrgba = LinSrgba::new(0.5225, 0.5225, 0.523);
    pub const TEAL: LinSrgba = LinSrgba::new(0.0, 0.2140, 0.214);
    pub const TOMATO: LinSrgba = LinSrgba::new(1.0, 0.1260, 0.064);
    pub const TURQUOISE: LinSrgba = LinSrgba::new(0.0509, 0.7484, 0.638);
    pub const VIOLET: LinSrgba = LinSrgba::new(0.8481, 0.2234, 0.848);
    pub const WHITE: LinSrgba = LinSrgba::new(1.0, 1.0, 1.0);
    pub const YELLOW: LinSrgba = LinSrgba::new(1.0, 1.0, 0.0);
    pub const YELLOW_GREEN: LinSrgba = LinSrgba::new(0.3185, 0.6038, 0.033);

    /// New [`Color`] from linear colorspace.
    pub const fn new(r: f32, g: f32, b: f32) -> LinSrgba {
        LinSrgba::with_alpha(r, g, b, 1.0)
    }

    /// New [`Color`] from linear colorspace.
    pub const fn with_alpha(r: f32, g: f32, b: f32, a: f32) -> LinSrgba {
        LinSrgba { r, g, b, a }
    }
}

impl Default for LinSrgba {
    fn default() -> Self {
        LinSrgba::WHITE
    }
}

impl AddAssign<LinSrgba> for LinSrgba {
    fn add_assign(&mut self, rhs: LinSrgba) {
        *self = LinSrgba {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
    }
}

impl Add<LinSrgba> for LinSrgba {
    type Output = LinSrgba;

    fn add(mut self, rhs: LinSrgba) -> Self::Output {
        self += rhs;
        self
    }
}

impl Mul<f32> for LinSrgba {
    type Output = LinSrgba;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<f32> for LinSrgba {
    fn mul_assign(&mut self, rhs: f32) {
        self.r *= rhs;
        self.g *= rhs;
        self.b *= rhs;
        //self.a *= rhs;
    }
}

impl Mul<LinSrgba> for LinSrgba {
    type Output = LinSrgba;

    fn mul(mut self, rhs: LinSrgba) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<LinSrgba> for LinSrgba {
    fn mul_assign(&mut self, rhs: LinSrgba) {
        self.r *= rhs.r;
        self.g *= rhs.g;
        self.b *= rhs.b;
        self.a *= rhs.a;
    }
}

impl From<LinSrgba> for [f32; 4] {
    fn from(color: LinSrgba) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}

impl From<[f32; 3]> for LinSrgba {
    fn from([r, g, b]: [f32; 3]) -> Self {
        LinSrgba::with_alpha(r, g, b, 1.0)
    }
}

impl From<[f32; 4]> for LinSrgba {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        LinSrgba::with_alpha(r, g, b, a)
    }
}

impl From<LinSrgba> for Vec4 {
    fn from(color: LinSrgba) -> Self {
        Vec4::new(color.r, color.g, color.b, color.a)
    }
}

impl From<Vec3> for LinSrgba {
    fn from(vec4: Vec3) -> Self {
        LinSrgba::with_alpha(vec4.x, vec4.y, vec4.z, 1.0)
    }
}

impl From<Vec4> for LinSrgba {
    fn from(vec4: Vec4) -> Self {
        LinSrgba::with_alpha(vec4.x, vec4.y, vec4.z, vec4.w)
    }
}

impl_render_resource_bytes!(LinSrgba);

#[test]
fn test_conversions_vec4() {
    let starting_vec4 = Vec4::new(0.4, 0.5, 0.6, 1.0);
    let starting_color = LinSrgba::from(starting_vec4);

    assert_eq!(starting_vec4, Vec4::from(starting_color),);
}

#[test]
fn test_mul_and_mulassign_f32() {
    let starting_color = LinSrgba::with_alpha(0.4, 0.5, 0.6, 1.0);
    assert_eq!(
        starting_color * 0.5,
        LinSrgba::with_alpha(0.4 * 0.5, 0.5 * 0.5, 0.6 * 0.5, 1.0),
    );
}
