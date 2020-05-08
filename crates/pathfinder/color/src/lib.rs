// pathfinder/color/src/lib.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use pathfinder_simd::default::F32x4;
use std::f32::consts::PI;
use std::fmt::{self, Debug, Formatter};
use std::slice;

// TODO(pcwalton): Maybe this should be a u32? Need to be aware of endianness issues if we do that.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(C)]
pub struct ColorU {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ColorU {
    #[inline]
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> ColorU {
        ColorU { r, g, b, a }
    }

    #[inline]
    pub fn transparent_black() -> ColorU {
        ColorU::from_u32(0)
    }

    #[inline]
    pub fn from_u32(rgba: u32) -> ColorU {
        ColorU {
            r: (rgba >> 24) as u8,
            g: ((rgba >> 16) & 0xff) as u8,
            b: ((rgba >> 8) & 0xff) as u8,
            a: (rgba & 0xff) as u8,
        }
    }

    #[inline]
    pub fn black() -> ColorU {
        ColorU {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    #[inline]
    pub fn white() -> ColorU {
        ColorU {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }
    }

    #[inline]
    pub fn to_f32(&self) -> ColorF {
        let color = F32x4::new(self.r as f32, self.g as f32, self.b as f32, self.a as f32);
        ColorF(color * F32x4::splat(1.0 / 255.0))
    }

    #[inline]
    pub fn is_opaque(&self) -> bool {
        self.a == !0
    }

    #[inline]
    pub fn is_fully_transparent(&self) -> bool {
        self.a == 0
    }
}

impl Debug for ColorU {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        if self.a == 255 {
            write!(formatter, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            write!(
                formatter,
                "rgba({}, {}, {}, {})",
                self.r,
                self.g,
                self.b,
                self.a as f32 / 255.0
            )
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub struct ColorF(pub F32x4);

impl ColorF {
    // Constructors

    #[inline]
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> ColorF {
        ColorF(F32x4::new(r, g, b, a))
    }

    #[inline]
    pub fn from_hsla(mut h: f32, s: f32, l: f32, a: f32) -> ColorF {
        // https://en.wikipedia.org/wiki/HSL_and_HSV#HSL_to_RGB

        // Make sure hue is always positive.
        h %= 2.0 * PI;
        if h < 0.0 {
            h += 2.0 * PI;
        }

        h *= 3.0 / PI;

        // Calculate chroma.
        let c = (1.0 - f32::abs(2.0 * l - 1.0)) * s;
        let xc = F32x4::new(c * (1.0 - f32::abs(h % 2.0 - 1.0)), c, 0.0, a);
        let rgba = match f32::ceil(h) as i32 {
            1     => xc.yxzw(),
            2     => xc.xyzw(),
            3     => xc.zyxw(),
            4     => xc.zxyw(),
            5     => xc.xzyw(),
            0 | 6 => xc.yzxw(),
            _     => xc.zzzw(),
        };
        let m = l - 0.5 * c;
        ColorF(rgba + F32x4::new(m, m, m, 0.0))
    }

    #[inline]
    pub fn from_hsl(h: f32, s: f32, l: f32) -> ColorF {
        ColorF::from_hsla(h, s, l, 1.0)
    }

    #[inline]
    pub fn transparent_black() -> ColorF {
        ColorF::default()
    }

    #[inline]
    pub fn black() -> ColorF {
        ColorF(F32x4::new(0.0, 0.0, 0.0, 1.0))
    }

    #[inline]
    pub fn white() -> ColorF {
        ColorF(F32x4::splat(1.0))
    }

    #[inline]
    pub fn to_u8(&self) -> ColorU {
        let color = (self.0 * F32x4::splat(255.0)).to_i32x4();
        ColorU { r: color[0] as u8, g: color[1] as u8, b: color[2] as u8, a: color[3] as u8 }
    }

    #[inline]
    pub fn lerp(&self, other: ColorF, t: f32) -> ColorF {
        ColorF(self.0 + (other.0 - self.0) * F32x4::splat(t))
    }

    #[inline]
    pub fn r(&self) -> f32 {
        self.0[0]
    }

    #[inline]
    pub fn g(&self) -> f32 {
        self.0[1]
    }

    #[inline]
    pub fn b(&self) -> f32 {
        self.0[2]
    }

    #[inline]
    pub fn a(&self) -> f32 {
        self.0[3]
    }

    #[inline]
    pub fn set_r(&mut self, r: f32) {
        self.0[0] = r;
    }

    #[inline]
    pub fn set_g(&mut self, g: f32) {
        self.0[1] = g;
    }

    #[inline]
    pub fn set_b(&mut self, b: f32) {
        self.0[2] = b;
    }

    #[inline]
    pub fn set_a(&mut self, a: f32) {
        self.0[3] = a;
    }
}

impl Debug for ColorF {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(
            formatter,
            "rgba({}, {}, {}, {})",
            self.r() * 255.0,
            self.g() * 255.0,
            self.b() * 255.0,
            self.a()
        )
    }
}

#[inline]
pub fn color_slice_to_u8_slice(slice: &[ColorU]) -> &[u8] {
    unsafe {
        slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len() * 4)
    }
}

#[inline]
pub fn u8_slice_to_color_slice(slice: &[u8]) -> &[ColorU] {
    unsafe {
        assert_eq!(slice.len() % 4, 0);
        slice::from_raw_parts(slice.as_ptr() as *const ColorU, slice.len() / 4)
    }
}

// TODO(pcwalton): Do this without a copy?
#[inline]
pub fn u8_vec_to_color_vec(buffer: Vec<u8>) -> Vec<ColorU> {
    u8_slice_to_color_slice(&buffer).to_vec()
}

/// A convenience method to construct a `ColorU` from an RGB triple.
///
/// Alpha is set to 255.
#[inline]
pub fn rgbu(r: u8, g: u8, b: u8) -> ColorU {
    ColorU::new(r, g, b, 255)
}

/// A convenience method to construct a `ColorU` from an RGBA triple.
#[inline]
pub fn rgbau(r: u8, g: u8, b: u8, a: u8) -> ColorU {
    ColorU::new(r, g, b, a)
}

/// A convenience method to construct a `ColorF` from an RGB triple.
///
/// Alpha is set to 1.0.
#[inline]
pub fn rgbf(r: f32, g: f32, b: f32) -> ColorF {
    ColorF::new(r, g, b, 1.0)
}

/// A convenience method to construct a `ColorF` from an RGBA triple.
#[inline]
pub fn rgbaf(r: f32, g: f32, b: f32, a: f32) -> ColorF {
    ColorF::new(r, g, b, a)
}
