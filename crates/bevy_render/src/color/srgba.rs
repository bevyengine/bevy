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

/// RGBA color in the sRGB colorspace
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct Srgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

unsafe impl Byteable for Srgba {}

impl Srgba {
    pub const ALICE_BLUE: Srgba = Srgba::rgb(0.94, 0.97, 1.0);
    pub const ANTIQUE_WHITE: Srgba = Srgba::rgb(0.98, 0.92, 0.84);
    pub const AQUAMARINE: Srgba = Srgba::rgb(0.49, 1.0, 0.83);
    pub const AZURE: Srgba = Srgba::rgb(0.94, 1.0, 1.0);
    pub const BEIGE: Srgba = Srgba::rgb(0.96, 0.96, 0.86);
    pub const BISQUE: Srgba = Srgba::rgb(1.0, 0.89, 0.77);
    pub const BLACK: Srgba = Srgba::rgb(0.0, 0.0, 0.0);
    pub const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);
    pub const CRIMSON: Srgba = Srgba::rgb(0.86, 0.08, 0.24);
    pub const CYAN: Srgba = Srgba::rgb(0.0, 1.0, 1.0);
    pub const DARK_GRAY: Srgba = Srgba::rgb(0.25, 0.25, 0.25);
    pub const DARK_GREEN: Srgba = Srgba::rgb(0.0, 0.5, 0.0);
    pub const FUCHSIA: Srgba = Srgba::rgb(1.0, 0.0, 1.0);
    pub const GOLD: Srgba = Srgba::rgb(1.0, 0.84, 0.0);
    pub const GRAY: Srgba = Srgba::rgb(0.5, 0.5, 0.5);
    pub const GREEN: Srgba = Srgba::rgb(0.0, 1.0, 0.0);
    pub const INDIGO: Srgba = Srgba::rgb(0.29, 0.0, 0.51);
    pub const LIME_GREEN: Srgba = Srgba::rgb(0.2, 0.8, 0.2);
    pub const MAROON: Srgba = Srgba::rgb(0.5, 0.0, 0.0);
    pub const MIDNIGHT_BLUE: Srgba = Srgba::rgb(0.1, 0.1, 0.44);
    pub const NAVY: Srgba = Srgba::rgb(0.0, 0.0, 0.5);
    pub const NONE: Srgba = Srgba::rgba(0.0, 0.0, 0.0, 0.0);
    pub const OLIVE: Srgba = Srgba::rgb(0.5, 0.5, 0.0);
    pub const ORANGE: Srgba = Srgba::rgb(1.0, 0.65, 0.0);
    pub const ORANGE_RED: Srgba = Srgba::rgb(1.0, 0.27, 0.0);
    pub const PINK: Srgba = Srgba::rgb(1.0, 0.08, 0.58);
    pub const PURPLE: Srgba = Srgba::rgb(0.5, 0.0, 0.5);
    pub const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
    pub const SALMON: Srgba = Srgba::rgb(0.98, 0.5, 0.45);
    pub const SEA_GREEN: Srgba = Srgba::rgb(0.18, 0.55, 0.34);
    pub const SILVER: Srgba = Srgba::rgb(0.75, 0.75, 0.75);
    pub const TEAL: Srgba = Srgba::rgb(0.0, 0.5, 0.5);
    pub const TOMATO: Srgba = Srgba::rgb(1.0, 0.39, 0.28);
    pub const TURQUOISE: Srgba = Srgba::rgb(0.25, 0.88, 0.82);
    pub const VIOLET: Srgba = Srgba::rgb(0.93, 0.51, 0.93);
    pub const WHITE: Srgba = Srgba::rgb(1.0, 1.0, 1.0);
    pub const YELLOW: Srgba = Srgba::rgb(1.0, 1.0, 0.0);
    pub const YELLOW_GREEN: Srgba = Srgba::rgb(0.6, 0.8, 0.);

    /// New [`Color`] from linear colorspace.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Srgba {
        Srgba::rgba(r, g, b, 1.0)
    }

    /// New [`Color`] from linear colorspace.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Srgba {
        Srgba { r, g, b, a }
    }

    /// New [`Color`] from sRGB colorspace.
    pub fn rgb24(r: u8, g: u8, b: u8) -> Srgba {
        Srgba::rgba32(r, g, b, u8::MAX)
    }

    // Float operations in const fn are not stable yet
    // see https://github.com/rust-lang/rust/issues/57241
    /// New [`Color`] from sRGB colorspace.
    pub fn rgba32(r: u8, g: u8, b: u8, a: u8) -> Srgba {
        Srgba::rgba(
            r as f32 / u8::MAX as f32,
            g as f32 / u8::MAX as f32,
            b as f32 / u8::MAX as f32,
            a as f32 / u8::MAX as f32,
        )
    }

    /// New [`Color`] from sRGB colorspace.
    pub fn from_hex<T: AsRef<str>>(hex: T) -> Result<Srgba, HexColorError> {
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
}

impl Default for Srgba {
    fn default() -> Self {
        Srgba::WHITE
    }
}

impl AddAssign<Srgba> for Srgba {
    fn add_assign(&mut self, rhs: Srgba) {
        *self = Srgba {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
    }
}

impl Add<Srgba> for Srgba {
    type Output = Srgba;

    fn add(mut self, rhs: Srgba) -> Self::Output {
        self += rhs;
        self
    }
}

impl Mul<f32> for Srgba {
    type Output = Srgba;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<f32> for Srgba {
    fn mul_assign(&mut self, rhs: f32) {
        self.r *= rhs;
        self.g *= rhs;
        self.b *= rhs;
        //self.a *= rhs;
    }
}

impl Mul<Srgba> for Srgba {
    type Output = Srgba;

    fn mul(mut self, rhs: Srgba) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<Srgba> for Srgba {
    fn mul_assign(&mut self, rhs: Srgba) {
        self.r *= rhs.r;
        self.g *= rhs.g;
        self.b *= rhs.b;
        self.a *= rhs.a;
    }
}

impl From<Srgba> for [f32; 4] {
    fn from(color: Srgba) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}

impl From<[f32; 3]> for Srgba {
    fn from([r, g, b]: [f32; 3]) -> Self {
        Srgba::rgba(r, g, b, 1.0)
    }
}

impl From<[f32; 4]> for Srgba {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Srgba::rgba(r, g, b, a)
    }
}

impl From<Srgba> for Vec4 {
    fn from(color: Srgba) -> Self {
        Vec4::new(color.r, color.g, color.b, color.a)
    }
}

impl From<Vec3> for Srgba {
    fn from(vec4: Vec3) -> Self {
        Srgba::rgba(vec4.x, vec4.y, vec4.z, 1.0)
    }
}

impl From<Vec4> for Srgba {
    fn from(vec4: Vec4) -> Self {
        Srgba::rgba(vec4.x, vec4.y, vec4.z, vec4.w)
    }
}

impl_render_resource_bytes!(Srgba);

#[derive(Debug)]
pub enum HexColorError {
    Length,
    Hex(base16::DecodeError),
}

fn decode_rgb(data: &[u8]) -> Result<Srgba, HexColorError> {
    let mut buf = [0; 3];
    match base16::decode_slice(data, &mut buf) {
        Ok(_) => {
            let r = buf[0] as f32 / 255.0;
            let g = buf[1] as f32 / 255.0;
            let b = buf[2] as f32 / 255.0;
            Ok(Srgba::rgb(r, g, b))
        }
        Err(err) => Err(HexColorError::Hex(err)),
    }
}

fn decode_rgba(data: &[u8]) -> Result<Srgba, HexColorError> {
    let mut buf = [0; 4];
    match base16::decode_slice(data, &mut buf) {
        Ok(_) => {
            let r = buf[0] as f32 / 255.0;
            let g = buf[1] as f32 / 255.0;
            let b = buf[2] as f32 / 255.0;
            let a = buf[3] as f32 / 255.0;
            Ok(Srgba::rgba(r, g, b, a))
        }
        Err(err) => Err(HexColorError::Hex(err)),
    }
}

#[test]
fn test_hex_color() {
    assert_eq!(Srgba::from_hex("FFF").unwrap(), Srgba::rgb(1.0, 1.0, 1.0));
    assert_eq!(Srgba::from_hex("000").unwrap(), Srgba::rgb(0.0, 0.0, 0.0));
    assert!(Srgba::from_hex("---").is_err());

    assert_eq!(
        Srgba::from_hex("FFFF").unwrap(),
        Srgba::rgba(1.0, 1.0, 1.0, 1.0)
    );
    assert_eq!(
        Srgba::from_hex("0000").unwrap(),
        Srgba::rgba(0.0, 0.0, 0.0, 0.0)
    );
    assert!(Srgba::from_hex("----").is_err());

    assert_eq!(
        Srgba::from_hex("FFFFFF").unwrap(),
        Srgba::rgb(1.0, 1.0, 1.0)
    );
    assert_eq!(
        Srgba::from_hex("000000").unwrap(),
        Srgba::rgb(0.0, 0.0, 0.0)
    );
    assert!(Srgba::from_hex("------").is_err());

    assert_eq!(
        Srgba::from_hex("FFFFFFFF").unwrap(),
        Srgba::rgba(1.0, 1.0, 1.0, 1.0)
    );
    assert_eq!(
        Srgba::from_hex("00000000").unwrap(),
        Srgba::rgba(0.0, 0.0, 0.0, 0.0)
    );
    assert!(Srgba::from_hex("--------").is_err());

    assert!(Srgba::from_hex("1234567890").is_err());
}

#[test]
fn test_conversions_vec4() {
    let starting_vec4 = Vec4::new(0.4, 0.5, 0.6, 1.0);
    let starting_color = Srgba::from(starting_vec4);

    assert_eq!(starting_vec4, Vec4::from(starting_color),);
}

#[test]
fn test_mul_and_mulassign_f32() {
    let starting_color = Srgba::rgba(0.4, 0.5, 0.6, 1.0);
    assert_eq!(
        starting_color * 0.5,
        Srgba::rgba(0.4 * 0.5, 0.5 * 0.5, 0.6 * 0.5, 1.0),
    );
}
