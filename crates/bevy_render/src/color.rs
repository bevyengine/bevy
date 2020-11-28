use super::texture::Texture;
use crate::{
    colorspace::*,
    impl_render_resource_bytes,
    renderer::{RenderResource, RenderResourceType},
};
use bevy_asset::Handle;
use bevy_core::{Byteable, Bytes};
use bevy_math::{Vec3, Vec4};
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, MulAssign};

// TODO: Separate types for non-linear sRGB and linear sRGB, with conversions between
// see comment on bevy issue #688 https://github.com/bevyengine/bevy/pull/688#issuecomment-711414011
/// RGBA color in the Linear sRGB colorspace (often colloquially referred to as "linear", "RGB", or "linear RGB").
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Color {
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
}

unsafe impl Byteable for Color {}

impl Color {
    pub const ALICE_BLUE: Color = Color::rgb_linear(0.94, 0.97, 1.0);
    pub const ANTIQUE_WHITE: Color = Color::rgb_linear(0.98, 0.92, 0.84);
    pub const AQUAMARINE: Color = Color::rgb_linear(0.49, 1.0, 0.83);
    pub const AZURE: Color = Color::rgb_linear(0.94, 1.0, 1.0);
    pub const BEIGE: Color = Color::rgb_linear(0.96, 0.96, 0.86);
    pub const BISQUE: Color = Color::rgb_linear(1.0, 0.89, 0.77);
    pub const BLACK: Color = Color::rgb_linear(0.0, 0.0, 0.0);
    pub const BLUE: Color = Color::rgb_linear(0.0, 0.0, 1.0);
    pub const CRIMSON: Color = Color::rgb_linear(0.86, 0.08, 0.24);
    pub const CYAN: Color = Color::rgb_linear(0.0, 1.0, 1.0);
    pub const DARK_GRAY: Color = Color::rgb_linear(0.25, 0.25, 0.25);
    pub const DARK_GREEN: Color = Color::rgb_linear(0.0, 0.5, 0.0);
    pub const FUCHSIA: Color = Color::rgb_linear(1.0, 0.0, 1.0);
    pub const GOLD: Color = Color::rgb_linear(1.0, 0.84, 0.0);
    pub const GRAY: Color = Color::rgb_linear(0.5, 0.5, 0.5);
    pub const GREEN: Color = Color::rgb_linear(0.0, 1.0, 0.0);
    pub const INDIGO: Color = Color::rgb_linear(0.29, 0.0, 0.51);
    pub const LIME_GREEN: Color = Color::rgb_linear(0.2, 0.8, 0.2);
    pub const MAROON: Color = Color::rgb_linear(0.5, 0.0, 0.0);
    pub const MIDNIGHT_BLUE: Color = Color::rgb_linear(0.1, 0.1, 0.44);
    pub const NAVY: Color = Color::rgb_linear(0.0, 0.0, 0.5);
    pub const NONE: Color = Color::rgba_linear(0.0, 0.0, 0.0, 0.0);
    pub const OLIVE: Color = Color::rgb_linear(0.5, 0.5, 0.0);
    pub const ORANGE: Color = Color::rgb_linear(1.0, 0.65, 0.0);
    pub const ORANGE_RED: Color = Color::rgb_linear(1.0, 0.27, 0.0);
    pub const PINK: Color = Color::rgb_linear(1.0, 0.08, 0.58);
    pub const PURPLE: Color = Color::rgb_linear(0.5, 0.0, 0.5);
    pub const RED: Color = Color::rgb_linear(1.0, 0.0, 0.0);
    pub const SALMON: Color = Color::rgb_linear(0.98, 0.5, 0.45);
    pub const SEA_GREEN: Color = Color::rgb_linear(0.18, 0.55, 0.34);
    pub const SILVER: Color = Color::rgb_linear(0.75, 0.75, 0.75);
    pub const TEAL: Color = Color::rgb_linear(0.0, 0.5, 0.5);
    pub const TOMATO: Color = Color::rgb_linear(1.0, 0.39, 0.28);
    pub const TURQUOISE: Color = Color::rgb_linear(0.25, 0.88, 0.82);
    pub const VIOLET: Color = Color::rgb_linear(0.93, 0.51, 0.93);
    pub const WHITE: Color = Color::rgb_linear(1.0, 1.0, 1.0);
    pub const YELLOW: Color = Color::rgb_linear(1.0, 1.0, 0.0);
    pub const YELLOW_GREEN: Color = Color::rgb_linear(0.6, 0.8, 0.2);

    // TODO: cant make rgb and rgba const due traits not allowed in const functions
    // see issue #57563 https://github.com/rust-lang/rust/issues/57563
    /// New ``Color`` from sRGB colorspace.
    pub fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color {
            red: r,
            green: g,
            blue: b,
            alpha: 1.0,
        }
        .as_nonlinear_srgb_to_linear_srgb()
    }

    /// New ``Color`` from sRGB colorspace.
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
        .as_nonlinear_srgb_to_linear_srgb()
    }

    /// New ``Color`` from linear colorspace.
    pub const fn rgb_linear(r: f32, g: f32, b: f32) -> Color {
        Color {
            red: r,
            green: g,
            blue: b,
            alpha: 1.0,
        }
    }

    /// New ``Color`` from linear colorspace.
    pub const fn rgba_linear(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
    }

    /// New ``Color`` from sRGB colorspace.
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

    /// New ``Color`` from sRGB colorspace.
    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Color {
        Color::rgba_u8(r, g, b, u8::MAX)
    }

    // Float operations in const fn are not stable yet
    // see https://github.com/rust-lang/rust/issues/57241
    /// New ``Color`` from sRGB colorspace.
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
            red: self.red.nonlinear_to_linear_srgb(),
            green: self.green.nonlinear_to_linear_srgb(),
            blue: self.blue.nonlinear_to_linear_srgb(),
            alpha: self.alpha, // alpha is always linear
        }
    }

    // non-linear-sRGB Component Getter

    /// Get red in sRGB colorspace.
    pub fn r(&self) -> f32 {
        self.red.linear_to_nonlinear_srgb()
    }

    /// Get green in sRGB colorspace.
    pub fn g(&self) -> f32 {
        self.green.linear_to_nonlinear_srgb()
    }

    /// Get blue in sRGB colorspace.
    pub fn b(&self) -> f32 {
        self.blue.linear_to_nonlinear_srgb()
    }

    // linear-sRGB Component Getter

    /// Get red in linear colorspace.
    pub fn r_linear(&self) -> f32 {
        self.red
    }

    /// Get green in linear colorspace.
    pub fn g_linear(&self) -> f32 {
        self.green
    }

    /// Get blue in linear colorspace.
    pub fn b_linear(&self) -> f32 {
        self.blue
    }

    /// Get alpha.
    pub fn a(&self) -> f32 {
        self.alpha
    }

    // non-linear-sRGB Component Setter

    /// Set red in sRGB colorspace.
    pub fn set_r(&mut self, r: f32) -> &mut Self {
        self.red = r.nonlinear_to_linear_srgb();
        self
    }

    /// Set green in sRGB colorspace.
    pub fn set_g(&mut self, g: f32) -> &mut Self {
        self.green = g.nonlinear_to_linear_srgb();
        self
    }

    /// Set blue in sRGB colorspace.
    pub fn set_b(&mut self, b: f32) -> &mut Self {
        self.blue = b.nonlinear_to_linear_srgb();
        self
    }

    // linear-sRGB Component Setter

    /// Set red in linear colorspace.
    pub fn set_r_linear(&mut self, r: f32) -> &mut Self {
        self.red = r;
        self
    }

    /// Set green in linear colorspace.
    pub fn set_g_linear(&mut self, g: f32) -> &mut Self {
        self.green = g;
        self
    }

    /// Set blue in linear colorspace.
    pub fn set_b_linear(&mut self, b: f32) -> &mut Self {
        self.blue = b;
        self
    }

    /// Set alpha.
    pub fn set_a(&mut self, a: f32) -> &mut Self {
        self.alpha = a;
        self
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
            red: self.red + rhs.red,
            green: self.green + rhs.green,
            blue: self.blue + rhs.blue,
            alpha: self.alpha + rhs.alpha,
        }
    }
}

impl Add<Color> for Color {
    type Output = Color;

    fn add(self, rhs: Color) -> Self::Output {
        Color {
            red: self.red + rhs.red,
            green: self.green + rhs.green,
            blue: self.blue + rhs.blue,
            alpha: self.alpha + rhs.alpha,
        }
    }
}

impl Add<Vec4> for Color {
    type Output = Color;

    fn add(self, rhs: Vec4) -> Self::Output {
        Color {
            red: self.red + rhs.x,
            green: self.green + rhs.y,
            blue: self.blue + rhs.z,
            alpha: self.alpha + rhs.w,
        }
    }
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> Self {
        [color.r(), color.g(), color.b(), color.a()]
    }
}

impl From<[f32; 4]> for Color {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Color::rgba(r, g, b, a)
    }
}

impl From<Color> for Vec4 {
    fn from(color: Color) -> Self {
        Vec4::new(color.r(), color.g(), color.b(), color.a())
    }
}

impl From<Vec4> for Color {
    fn from(vec4: Vec4) -> Self {
        Color::rgba(vec4.x, vec4.y, vec4.z, vec4.w)
    }
}

impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Self::Output {
        Color::rgba(self.r() * rhs, self.g() * rhs, self.b() * rhs, self.a())
    }
}

impl MulAssign<f32> for Color {
    fn mul_assign(&mut self, rhs: f32) {
        self.set_r(self.r() * rhs);
        self.set_g(self.g() * rhs);
        self.set_b(self.b() * rhs);
    }
}

impl Mul<Vec4> for Color {
    type Output = Color;

    fn mul(self, rhs: Vec4) -> Self::Output {
        Color::rgba(
            self.r() * rhs.x,
            self.g() * rhs.y,
            self.b() * rhs.z,
            self.a() * rhs.w,
        )
    }
}

impl MulAssign<Vec4> for Color {
    fn mul_assign(&mut self, rhs: Vec4) {
        self.set_r(self.r() * rhs.x);
        self.set_g(self.g() * rhs.y);
        self.set_b(self.b() * rhs.z);
        self.set_a(self.a() * rhs.w);
    }
}

impl Mul<Vec3> for Color {
    type Output = Color;

    fn mul(self, rhs: Vec3) -> Self::Output {
        Color::rgba(
            self.r() * rhs.x,
            self.g() * rhs.y,
            self.b() * rhs.z,
            self.a(),
        )
    }
}

impl MulAssign<Vec3> for Color {
    fn mul_assign(&mut self, rhs: Vec3) {
        self.set_r(self.r() * rhs.x);
        self.set_g(self.g() * rhs.y);
        self.set_b(self.b() * rhs.z);
    }
}

impl Mul<[f32; 4]> for Color {
    type Output = Color;

    fn mul(self, [r, g, b, a]: [f32; 4]) -> Self::Output {
        Color::rgba(self.r() * r, self.g() * g, self.b() * b, self.a() * a)
    }
}

impl MulAssign<[f32; 4]> for Color {
    fn mul_assign(&mut self, [r, g, b, a]: [f32; 4]) {
        self.set_r(self.r() * r);
        self.set_g(self.g() * g);
        self.set_b(self.b() * b);
        self.set_a(self.a() * a);
    }
}

impl Mul<[f32; 3]> for Color {
    type Output = Color;

    fn mul(self, [r, g, b]: [f32; 3]) -> Self::Output {
        Color::rgba(self.r() * r, self.g() * g, self.b() * b, self.a())
    }
}

impl MulAssign<[f32; 3]> for Color {
    fn mul_assign(&mut self, [r, g, b]: [f32; 3]) {
        self.set_r(self.r() * r);
        self.set_g(self.g() * g);
        self.set_b(self.b() * b);
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

impl From<[f32; 4]> for ColorSource {
    fn from(f32s: [f32; 4]) -> Self {
        ColorSource::Color(f32s.into())
    }
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
fn test_color_components_roundtrip() {
    let mut color = Color::NONE;
    color.set_r(0.5).set_g(0.5).set_b(0.5).set_a(0.5);
    const EPS: f32 = 0.001;
    assert!((color.r() - 0.5).abs() < EPS);
    assert!((color.g() - 0.5).abs() < EPS);
    assert!((color.b() - 0.5).abs() < EPS);
    assert!((color.a() - 0.5).abs() < EPS);
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

    let transformation = Vec4::new(0.5, 0.5, 0.5, 1.0);

    assert_eq!(
        starting_color * transformation,
        Color::from(starting_vec4 * transformation),
    );
}

#[test]
fn test_mul_and_mulassign_f32() {
    let transformation = 0.5;
    let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

    assert_eq!(
        starting_color * transformation,
        Color::rgba(0.4 * 0.5, 0.5 * 0.5, 0.6 * 0.5, 1.0),
    );

    let mut mutated_color = starting_color;
    mutated_color *= transformation;

    assert_eq!(starting_color * transformation, mutated_color,);
}

#[test]
fn test_mul_and_mulassign_f32by3() {
    let transformation = [0.4, 0.5, 0.6];
    let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

    assert_eq!(
        starting_color * transformation,
        Color::rgba(0.4 * 0.4, 0.5 * 0.5, 0.6 * 0.6, 1.0),
    );

    let mut mutated_color = starting_color;
    mutated_color *= transformation;

    assert_eq!(starting_color * transformation, mutated_color,);
}

#[test]
fn test_mul_and_mulassign_f32by4() {
    let transformation = [0.4, 0.5, 0.6, 0.9];
    let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

    assert_eq!(
        starting_color * transformation,
        Color::rgba(0.4 * 0.4, 0.5 * 0.5, 0.6 * 0.6, 1.0 * 0.9),
    );

    let mut mutated_color = starting_color;
    mutated_color *= transformation;

    assert_eq!(starting_color * transformation, mutated_color,);
}

#[test]
fn test_mul_and_mulassign_vec3() {
    let transformation = Vec3::new(0.2, 0.3, 0.4);
    let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

    assert_eq!(
        starting_color * transformation,
        Color::rgba(0.4 * 0.2, 0.5 * 0.3, 0.6 * 0.4, 1.0),
    );

    let mut mutated_color = starting_color;
    mutated_color *= transformation;

    assert_eq!(starting_color * transformation, mutated_color,);
}

#[test]
fn test_mul_and_mulassign_vec4() {
    let transformation = Vec4::new(0.2, 0.3, 0.4, 0.5);
    let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

    assert_eq!(
        starting_color * transformation,
        Color::rgba(0.4 * 0.2, 0.5 * 0.3, 0.6 * 0.4, 1.0 * 0.5),
    );

    let mut mutated_color = starting_color;
    mutated_color *= transformation;

    assert_eq!(starting_color * transformation, mutated_color,);
}
