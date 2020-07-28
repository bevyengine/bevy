use super::texture::Texture;
use crate::{
    impl_render_resource_bytes,
    renderer::{RenderResource, RenderResourceType},
};
use bevy_asset::Handle;
use bevy_core::{Byteable, Bytes};
use bevy_math::{Vec3, Vec4};
use bevy_property::Property;
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, MulAssign};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Property)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

unsafe impl Byteable for Color {}

impl Color {
    pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);
    pub const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);
    pub const GREEN: Color = Color::rgb(0.0, 1.0, 0.0);
    pub const NONE: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);
    pub const RED: Color = Color::rgb(1.0, 0.0, 0.0);
    pub const WHITE: Color = Color::rgb(1.0, 1.0, 1.0);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::WHITE
    }
}

impl AddAssign<Color> for Color {
    fn add_assign(&mut self, rhs: Color) {
        *self = Color {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
    }
}

impl Add<Color> for Color {
    type Output = Color;

    fn add(self, rhs: Color) -> Self::Output {
        Color {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
    }
}

impl Add<Vec4> for Color {
    type Output = Color;

    fn add(self, rhs: Vec4) -> Self::Output {
        Color {
            r: self.r + rhs.x(),
            g: self.g + rhs.y(),
            b: self.b + rhs.z(),
            a: self.a + rhs.w(),
        }
    }
}

impl From<Vec4> for Color {
    fn from(vec4: Vec4) -> Self {
        Color {
            r: vec4.x(),
            g: vec4.y(),
            b: vec4.z(),
            a: vec4.w(),
        }
    }
}

impl Into<[f32; 4]> for Color {
    fn into(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}
impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Self::Output {
        Color {
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            a: self.a * rhs,
        }
    }
}

impl MulAssign<f32> for Color {
    fn mul_assign(&mut self, rhs: f32) {
        self.r *= rhs;
        self.g *= rhs;
        self.b *= rhs;
        self.a *= rhs;
    }
}

impl Mul<Vec4> for Color {
    type Output = Color;

    fn mul(self, rhs: Vec4) -> Self::Output {
        Color {
            r: self.r * rhs.x(),
            g: self.g * rhs.y(),
            b: self.b * rhs.z(),
            a: self.a * rhs.w(),
        }
    }
}

impl MulAssign<Vec4> for Color {
    fn mul_assign(&mut self, rhs: Vec4) {
        self.r *= rhs.x();
        self.g *= rhs.y();
        self.b *= rhs.z();
        self.a *= rhs.w();
    }
}

impl Mul<Vec3> for Color {
    type Output = Color;

    fn mul(self, rhs: Vec3) -> Self::Output {
        Color {
            r: self.r * rhs.x(),
            g: self.g * rhs.y(),
            b: self.b * rhs.z(),
            a: self.a,
        }
    }
}

impl MulAssign<Vec3> for Color {
    fn mul_assign(&mut self, rhs: Vec3) {
        self.r *= rhs.x();
        self.g *= rhs.y();
        self.b *= rhs.z();
    }
}

impl Bytes for ColorSource {
    fn write_bytes(&self, buffer: &mut [u8]) {
        match *self {
            ColorSource::Color(ref color) => color.write_bytes(buffer),
            ColorSource::Texture(_) => {} // Texture is not a uniform
        }
    }

    fn byte_len(&self) -> usize {
        match *self {
            ColorSource::Color(ref color) => color.byte_len(),
            ColorSource::Texture(_) => 0, // Texture is not a uniform
        }
    }
}

pub enum ColorSource {
    Color(Color),
    Texture(Handle<Texture>),
}

impl From<Vec4> for ColorSource {
    fn from(vec4: Vec4) -> Self {
        ColorSource::Color(vec4.into())
    }
}

impl From<Color> for ColorSource {
    fn from(color: Color) -> Self {
        ColorSource::Color(color)
    }
}

impl From<Handle<Texture>> for ColorSource {
    fn from(texture: Handle<Texture>) -> Self {
        ColorSource::Texture(texture)
    }
}

impl_render_resource_bytes!(Color);
