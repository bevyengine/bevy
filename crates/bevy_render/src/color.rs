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

/// A RGBA color
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

    pub fn hex<T: AsRef<str>>(hex: T) -> Result<Color, HexColorError> {
        let hex = hex.as_ref();

        // RGB
        if hex.len() == 3 {
            let mut data = [0; 6];
            for (i, ch) in hex.chars().enumerate() {
                data[i * 2] = ch as u8;
                data[i * 2 + 1] = ch as u8;
            }
            return decode_rgb(&data);
        }

        // RGBA
        if hex.len() == 4 {
            let mut data = [0; 8];
            for (i, ch) in hex.chars().enumerate() {
                data[i * 2] = ch as u8;
                data[i * 2 + 1] = ch as u8;
            }
            return decode_rgba(&data);
        }

        // RRGGBB
        if hex.len() == 6 {
            return decode_rgb(hex.as_bytes());
        }

        // RRGGBBAA
        if hex.len() == 8 {
            return decode_rgba(hex.as_bytes());
        }

        Err(HexColorError::Length)
    }

    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Color {
        Color::rgba_u8(r, g, b, u8::MAX)
    }

    // Float operations in const fn are not stable yet
    // see https://github.com/rust-lang/rust/issues/57241
    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color::rgba(
            r as f32 / u8::MAX as f32,
            g as f32 / u8::MAX as f32,
            b as f32 / u8::MAX as f32,
            a as f32 / u8::MAX as f32,
        )
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

/// A source of color
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

#[derive(Debug)]
pub enum HexColorError {
    Length,
    Hex(hex::FromHexError),
}

fn decode_rgb(data: &[u8]) -> Result<Color, HexColorError> {
    let mut buf = [0; 3];
    match hex::decode_to_slice(data, &mut buf) {
        Ok(_) => {
            let r = buf[0] as f32 / 255.0;
            let g = buf[1] as f32 / 255.0;
            let b = buf[2] as f32 / 255.0;
            Ok(Color::rgb(r, g, b))
        }
        Err(err) => Err(HexColorError::Hex(err)),
    }
}

fn decode_rgba(data: &[u8]) -> Result<Color, HexColorError> {
    let mut buf = [0; 4];
    match hex::decode_to_slice(data, &mut buf) {
        Ok(_) => {
            let r = buf[0] as f32 / 255.0;
            let g = buf[1] as f32 / 255.0;
            let b = buf[2] as f32 / 255.0;
            let a = buf[3] as f32 / 255.0;
            Ok(Color::rgba(r, g, b, a))
        }
        Err(err) => Err(HexColorError::Hex(err)),
    }
}

#[test]
fn test_hex_color() {
    assert_eq!(Color::hex("FFF").unwrap(), Color::rgb(1.0, 1.0, 1.0));
    assert_eq!(Color::hex("000").unwrap(), Color::rgb(0.0, 0.0, 0.0));
    assert!(Color::hex("---").is_err());

    assert_eq!(Color::hex("FFFF").unwrap(), Color::rgba(1.0, 1.0, 1.0, 1.0));
    assert_eq!(Color::hex("0000").unwrap(), Color::rgba(0.0, 0.0, 0.0, 0.0));
    assert!(Color::hex("----").is_err());

    assert_eq!(Color::hex("FFFFFF").unwrap(), Color::rgb(1.0, 1.0, 1.0));
    assert_eq!(Color::hex("000000").unwrap(), Color::rgb(0.0, 0.0, 0.0));
    assert!(Color::hex("------").is_err());

    assert_eq!(
        Color::hex("FFFFFFFF").unwrap(),
        Color::rgba(1.0, 1.0, 1.0, 1.0)
    );
    assert_eq!(
        Color::hex("00000000").unwrap(),
        Color::rgba(0.0, 0.0, 0.0, 0.0)
    );
    assert!(Color::hex("--------").is_err());

    assert!(Color::hex("1234567890").is_err());
}
