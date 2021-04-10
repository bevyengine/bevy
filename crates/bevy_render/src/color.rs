use super::texture::Texture;
use crate::{
    colorspace::*,
    impl_render_resource_bytes,
    renderer::{RenderResource, RenderResourceType},
};
use bevy_asset::Handle;
use bevy_core::{Byteable, Bytes};
use bevy_math::{Vec3, Vec4};
use bevy_reflect::{Reflect, ReflectDeserialize};
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, MulAssign};

/// RGBA color in the Linear sRGB colorspace (often colloquially referred to as "linear", "RGB", or
/// "linear RGB").
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

unsafe impl Byteable for Color {}

impl Color {
    pub const ALICE_BLUE: Color = Color::rgb_linear(0.8689, 0.9331, 1.0);
    pub const ANTIQUE_WHITE: Color = Color::rgb_linear(0.9551, 0.8276, 0.674);
    pub const AQUAMARINE: Color = Color::rgb_linear(0.2049, 1.0, 0.656);
    pub const AZURE: Color = Color::rgb_linear(0.8689, 1.0, 1.0);
    pub const BEIGE: Color = Color::rgb_linear(0.9114, 0.9114, 0.711);
    pub const BISQUE: Color = Color::rgb_linear(1.0, 0.7678, 0.554);
    pub const BLACK: Color = Color::rgb_linear(0.0, 0.0, 0.0);
    pub const BLUE: Color = Color::rgb_linear(0.0, 0.0, 1.0);
    pub const CRIMSON: Color = Color::rgb_linear(0.7106, 0.0072, 0.047);
    pub const CYAN: Color = Color::rgb_linear(0.0, 1.0, 1.0);
    pub const DARK_GRAY: Color = Color::rgb_linear(0.0509, 0.0509, 0.051);
    pub const DARK_GREEN: Color = Color::rgb_linear(0.0, 0.2140, 0.0);
    pub const FUCHSIA: Color = Color::rgb_linear(1.0, 0.0, 1.0);
    pub const GOLD: Color = Color::rgb_linear(1.0, 0.6739, 0.0);
    pub const GRAY: Color = Color::rgb_linear(0.2140, 0.2140, 0.214);
    pub const GREEN: Color = Color::rgb_linear(0.0, 1.0, 0.0);
    pub const INDIGO: Color = Color::rgb_linear(0.0684, 0.0, 0.223);
    pub const LIME_GREEN: Color = Color::rgb_linear(0.0331, 0.6038, 0.033);
    pub const MAROON: Color = Color::rgb_linear(0.2140, 0.0, 0.0);
    pub const MIDNIGHT_BLUE: Color = Color::rgb_linear(0.0100, 0.0100, 0.163);
    pub const NAVY: Color = Color::rgb_linear(0.0, 0.0, 0.214);
    pub const NONE: Color = Color::rgba_linear(0.0, 0.0, 0.0, 0.0);
    pub const OLIVE: Color = Color::rgb_linear(0.2140, 0.2140, 0.0);
    pub const ORANGE: Color = Color::rgb_linear(1.0, 0.3801, 0.0);
    pub const ORANGE_RED: Color = Color::rgb_linear(1.0, 0.0593, 0.0);
    pub const PINK: Color = Color::rgb_linear(1.0, 0.0072, 0.296);
    pub const PURPLE: Color = Color::rgb_linear(0.2140, 0.0, 0.214);
    pub const RED: Color = Color::rgb_linear(1.0, 0.0, 0.0);
    pub const SALMON: Color = Color::rgb_linear(0.9551, 0.2140, 0.171);
    pub const SEA_GREEN: Color = Color::rgb_linear(0.0272, 0.2633, 0.095);
    pub const SILVER: Color = Color::rgb_linear(0.5225, 0.5225, 0.523);
    pub const TEAL: Color = Color::rgb_linear(0.0, 0.2140, 0.214);
    pub const TOMATO: Color = Color::rgb_linear(1.0, 0.1260, 0.064);
    pub const TURQUOISE: Color = Color::rgb_linear(0.0509, 0.7484, 0.638);
    pub const VIOLET: Color = Color::rgb_linear(0.8481, 0.2234, 0.848);
    pub const WHITE: Color = Color::rgb_linear(1.0, 1.0, 1.0);
    pub const YELLOW: Color = Color::rgb_linear(1.0, 1.0, 0.0);
    pub const YELLOW_GREEN: Color = Color::rgb_linear(0.3185, 0.6038, 0.033);

    // TODO: cant make rgb and rgba const due traits not allowed in const functions
    // see issue #57563 https://github.com/rust-lang/rust/issues/57563
    /// New [`Color`] from sRGB colorspace.
    pub fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b, a: 1.0 }.as_nonlinear_srgb_to_linear_srgb()
    }

    /// New [`Color`] from sRGB colorspace.
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }.as_nonlinear_srgb_to_linear_srgb()
    }

    /// New [`Color`] from linear colorspace.
    pub const fn rgb_linear(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b, a: 1.0 }
    }

    /// New [`Color`] from linear colorspace.
    pub const fn rgba_linear(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }

    /// New [`Color`] from sRGB colorspace.
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

    /// New [`Color`] from sRGB colorspace.
    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Color {
        Color::rgba_u8(r, g, b, u8::MAX)
    }

    // Float operations in const fn are not stable yet
    // see https://github.com/rust-lang/rust/issues/57241
    /// New [`Color`] from sRGB colorspace.
    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color::rgba(
            r as f32 / u8::MAX as f32,
            g as f32 / u8::MAX as f32,
            b as f32 / u8::MAX as f32,
            a as f32 / u8::MAX as f32,
        )
    }

    fn as_nonlinear_srgb_to_linear_srgb(self) -> Color {
        Color {
            r: self.r.nonlinear_to_linear_srgb(),
            g: self.g.nonlinear_to_linear_srgb(),
            b: self.b.nonlinear_to_linear_srgb(),
            a: self.a, // alpha is always linear
        }
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

    fn add(mut self, rhs: Color) -> Self::Output {
        self += rhs;
        self
    }
}

impl Mul<f32> for Color {
    type Output = Color;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<f32> for Color {
    fn mul_assign(&mut self, rhs: f32) {
        self.r *= rhs;
        self.g *= rhs;
        self.b *= rhs;
        //self.a *= rhs;
    }
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}

impl From<[f32; 4]> for Color {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Color::rgba(r, g, b, a)
    }
}

impl From<Color> for Vec4 {
    fn from(color: Color) -> Self {
        Vec4::new(color.r, color.g, color.b, color.a)
    }
}

impl From<Vec3> for Color {
    fn from(vec4: Vec3) -> Self {
        Color::rgba(vec4.x, vec4.y, vec4.z, 1.0)
    }
}

impl From<Vec4> for Color {
    fn from(vec4: Vec4) -> Self {
        Color::rgba(vec4.x, vec4.y, vec4.z, vec4.w)
    }
}

impl_render_resource_bytes!(Color);

#[derive(Debug)]
pub enum HexColorError {
    Length,
    Hex(base16::DecodeError),
}

fn decode_rgb(data: &[u8]) -> Result<Color, HexColorError> {
    let mut buf = [0; 3];
    match base16::decode_slice(data, &mut buf) {
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
    match base16::decode_slice(data, &mut buf) {
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

#[test]
fn test_conversions_vec4() {
    let starting_vec4 = Vec4::new(0.4, 0.5, 0.6, 1.0);
    let starting_color = Color::from(starting_vec4);

    assert_eq!(starting_vec4, Vec4::from(starting_color),);
}

#[test]
fn test_mul_and_mulassign_f32() {
    let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);
    assert_eq!(
        starting_color * 0.5,
        Color::rgba(0.4 * 0.5, 0.5 * 0.5, 0.6 * 0.5, 1.0),
    );
}
