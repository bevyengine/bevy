/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

/*!
Gamma correction lookup tables.

This is a port of Skia gamma LUT logic into Rust, used by WebRender.
*/
//#![warn(missing_docs)] //TODO
#![allow(dead_code)]

use std::cmp::max;

use ColorU;

/// Color space responsible for converting between lumas and luminances.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LuminanceColorSpace {
    /// Linear space - no conversion involved.
    Linear,
    /// Simple gamma space - uses the `luminance ^ gamma` function.
    Gamma(f32),
    /// Srgb space.
    Srgb,
}

impl LuminanceColorSpace {
    pub fn new(gamma: f32) -> LuminanceColorSpace {
        if gamma == 1.0 {
            LuminanceColorSpace::Linear
        } else if gamma == 0.0 {
            LuminanceColorSpace::Srgb
        } else {
            LuminanceColorSpace::Gamma(gamma)
        }
    }

    pub fn to_luma(&self, luminance: f32) -> f32 {
        match *self {
            LuminanceColorSpace::Linear => luminance,
            LuminanceColorSpace::Gamma(gamma) => luminance.powf(gamma),
            LuminanceColorSpace::Srgb => {
                //The magic numbers are derived from the sRGB specification.
                //See http://www.color.org/chardata/rgb/srgb.xalter .
                if luminance <= 0.04045 {
                    luminance / 12.92
                } else {
                    ((luminance + 0.055) / 1.055).powf(2.4)
                }
            }
        }
    }

    pub fn from_luma(&self, luma: f32) -> f32 {
        match *self {
            LuminanceColorSpace::Linear => luma,
            LuminanceColorSpace::Gamma(gamma) => luma.powf(1. / gamma),
            LuminanceColorSpace::Srgb => {
                //The magic numbers are derived from the sRGB specification.
                //See http://www.color.org/chardata/rgb/srgb.xalter .
                if luma <= 0.0031308 {
                    luma * 12.92
                } else {
                    1.055 * luma.powf(1./2.4) - 0.055
                }
            }
        }
    }
}

//TODO: tests
fn round_to_u8(x : f32) -> u8 {
    let v = (x + 0.5).floor() as i32;
    assert!(0 <= v && v < 0x100);
    v as u8
}

//TODO: tests
/*
 * Scales base <= 2^N-1 to 2^8-1
 * @param N [1, 8] the number of bits used by base.
 * @param base the number to be scaled to [0, 255].
 */
fn scale255(n: u8, mut base: u8) -> u8 {
    base <<= 8 - n;
    let mut lum = base;
    let mut i = n;

    while i < 8 {
        lum |= base >> i;
        i += n;
    }

    lum
}

// Computes the luminance from the given r, g, and b in accordance with
// SK_LUM_COEFF_X. For correct results, r, g, and b should be in linear space.
fn compute_luminance(r: u8, g: u8, b: u8) -> u8 {
    // The following is
    // r * SK_LUM_COEFF_R + g * SK_LUM_COEFF_G + b * SK_LUM_COEFF_B
    // with SK_LUM_COEFF_X in 1.8 fixed point (rounding adjusted to sum to 256).
    let val: u32 = r as u32 * 54 + g as u32 * 183 + b as u32 * 19;
    assert!(val < 0x10000);
    (val >> 8) as u8
}

// Skia uses 3 bits per channel for luminance.
const LUM_BITS: u8 = 3;
// Mask of the highest used bits.
const LUM_MASK: u8 = ((1 << LUM_BITS) - 1) << (8 - LUM_BITS);

pub trait ColorLut {
    fn quantize(&self) -> ColorU;
    fn quantized_floor(&self) -> ColorU;
    fn quantized_ceil(&self) -> ColorU;
    fn luminance(&self) -> u8;
    fn luminance_color(&self) -> ColorU;
}

impl ColorLut for ColorU {
    // Compute a canonical color that is equivalent to the input color
    // for preblend table lookups. The alpha channel is never used for
    // preblending, so overwrite it with opaque.
    fn quantize(&self) -> ColorU {
        ColorU::new(
            scale255(LUM_BITS, self.r >> (8 - LUM_BITS)),
            scale255(LUM_BITS, self.g >> (8 - LUM_BITS)),
            scale255(LUM_BITS, self.b >> (8 - LUM_BITS)),
            255,
        )
    }

    // Quantize to the smallest value that yields the same table index.
    fn quantized_floor(&self) -> ColorU {
        ColorU::new(
            self.r & LUM_MASK,
            self.g & LUM_MASK,
            self.b & LUM_MASK,
            255,
        )
    }

    // Quantize to the largest value that yields the same table index.
    fn quantized_ceil(&self) -> ColorU {
        ColorU::new(
            self.r | !LUM_MASK,
            self.g | !LUM_MASK,
            self.b | !LUM_MASK,
            255,
        )
    }

    // Compute a luminance value suitable for grayscale preblend table
    // lookups.
    fn luminance(&self) -> u8 {
        compute_luminance(self.r, self.g, self.b)
    }

    // Make a grayscale color from the computed luminance.
    fn luminance_color(&self) -> ColorU {
        let lum = self.luminance();
        ColorU::new(lum, lum, lum, self.a)
    }
}

// A value of 0.5 for SK_GAMMA_CONTRAST appears to be a good compromise.
// With lower values small text appears washed out (though correctly so).
// With higher values lcd fringing is worse and the smoothing effect of
// partial coverage is diminished.
fn apply_contrast(srca: f32, contrast: f32) -> f32 {
    srca + ((1.0 - srca) * contrast * srca)
}

// The approach here is not necessarily the one with the lowest error
// See https://bel.fi/alankila/lcd/alpcor.html for a similar kind of thing
// that just search for the adjusted alpha value
pub fn build_gamma_correcting_lut(table: &mut [u8; 256], src: u8, contrast: f32,
                                  src_space: LuminanceColorSpace,
                                  dst_convert: LuminanceColorSpace) {

    let src = src as f32 / 255.0;
    let lin_src = src_space.to_luma(src);
    // Guess at the dst. The perceptual inverse provides smaller visual
    // discontinuities when slight changes to desaturated colors cause a channel
    // to map to a different correcting lut with neighboring srcI.
    // See https://code.google.com/p/chromium/issues/detail?id=141425#c59 .
    let dst = 1.0 - src;
    let lin_dst = dst_convert.to_luma(dst);

    // Contrast value tapers off to 0 as the src luminance becomes white
    let adjusted_contrast = contrast * lin_dst;

    // Remove discontinuity and instability when src is close to dst.
    // The value 1/256 is arbitrary and appears to contain the instability.
    if (src - dst).abs() < (1.0 / 256.0) {
        let mut ii : f32 = 0.0;
        for v in table.iter_mut() {
            let raw_srca = ii / 255.0;
            let srca = apply_contrast(raw_srca, adjusted_contrast);

            *v = round_to_u8(255.0 * srca);
            ii += 1.0;
        }
    } else {
        // Avoid slow int to float conversion.
        let mut ii : f32 = 0.0;
        for v in table.iter_mut() {
            // 'raw_srca += 1.0f / 255.0f' and even
            // 'raw_srca = i * (1.0f / 255.0f)' can add up to more than 1.0f.
            // When this happens the table[255] == 0x0 instead of 0xff.
            // See http://code.google.com/p/chromium/issues/detail?id=146466
            let raw_srca = ii / 255.0;
            let srca = apply_contrast(raw_srca, adjusted_contrast);
            assert!(srca <= 1.0);
            let dsta = 1.0 - srca;

            // Calculate the output we want.
            let lin_out = lin_src * srca + dsta * lin_dst;
            assert!(lin_out <= 1.0);
            let out = dst_convert.from_luma(lin_out);

            // Undo what the blit blend will do.
            // i.e. given the formula for OVER: out = src * result + (1 - result) * dst
            // solving for result gives:
            let result = (out - dst) / (src - dst);

            *v = round_to_u8(255.0 * result);
            debug!("Setting {:?} to {:?}", ii as u8, *v);

            ii += 1.0;
        }
    }
}

pub struct GammaLut {
    pub tables: [[u8; 256]; 1 << LUM_BITS],
}

impl GammaLut {
    // Skia actually makes 9 gamma tables, then based on the luminance color,
    // fetches the RGB gamma table for that color.
    fn generate_tables(&mut self, contrast: f32, paint_gamma: f32, device_gamma: f32) {
        let paint_color_space = LuminanceColorSpace::new(paint_gamma);
        let device_color_space = LuminanceColorSpace::new(device_gamma);

        for (i, entry) in self.tables.iter_mut().enumerate() {
            let luminance = scale255(LUM_BITS, i as u8);
            build_gamma_correcting_lut(entry,
                                       luminance,
                                       contrast,
                                       paint_color_space,
                                       device_color_space);
        }
    }

    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    pub fn get_table(&self, color: u8) -> &[u8; 256] {
        &self.tables[(color >> (8 - LUM_BITS)) as usize]
    }

    pub fn new(contrast: f32, paint_gamma: f32, device_gamma: f32) -> GammaLut {
        let mut table = GammaLut {
            tables: [[0; 256]; 1 << LUM_BITS],
        };

        table.generate_tables(contrast, paint_gamma, device_gamma);

        table
    }

    // Assumes pixels are in BGRA format. Assumes pixel values are in linear space already.
    pub fn preblend(&self, pixels: &mut [u8], color: ColorU) {
        let table_r = self.get_table(color.r);
        let table_g = self.get_table(color.g);
        let table_b = self.get_table(color.b);

        for pixel in pixels.chunks_mut(4) {
            let (b, g, r) = (table_b[pixel[0] as usize], table_g[pixel[1] as usize], table_r[pixel[2] as usize]);
            pixel[0] = b;
            pixel[1] = g;
            pixel[2] = r;
            pixel[3] = max(max(b, g), r);
        }
    }

    // Assumes pixels are in BGRA format. Assumes pixel values are in linear space already.
    pub fn preblend_grayscale(&self, pixels: &mut [u8], color: ColorU) {
        let table_g = self.get_table(color.g);

        for pixel in pixels.chunks_mut(4) {
            let luminance = compute_luminance(pixel[2], pixel[1], pixel[0]);
            let alpha = table_g[luminance as usize];
            pixel[0] = alpha;
            pixel[1] = alpha;
            pixel[2] = alpha;
            pixel[3] = alpha;
        }
    }

} // end impl GammaLut

#[cfg(test)]
mod tests {
    use super::*;

    fn over(dst: u32, src: u32, alpha: u32) -> u32 {
        (src * alpha + dst * (255 - alpha))/255
    }

    fn overf(dst: f32, src: f32, alpha: f32) -> f32 {
        ((src * alpha + dst * (255. - alpha))/255.) as f32
    }


    fn absdiff(a: u32, b: u32) -> u32 {
        if a < b  { b - a } else { a - b }
    }

    #[test]
    fn gamma() {
        let mut table = [0u8; 256];
        let g = 2.0;
        let space = LuminanceColorSpace::Gamma(g);
        let mut src : u32 = 131;
        while src < 256 {
            build_gamma_correcting_lut(&mut table, src as u8, 0., space, space);
            let mut max_diff = 0;
            let mut dst = 0;
            while dst < 256 {
                for alpha in 0u32..256 {
                    let preblend = table[alpha as usize];
                    let lin_dst = (dst as f32 / 255.).powf(g) * 255.;
                    let lin_src = (src as f32 / 255.).powf(g) * 255.;

                    let preblend_result = over(dst, src, preblend as u32);
                    let true_result = ((overf(lin_dst, lin_src, alpha as f32) / 255.).powf(1. / g) * 255.) as u32;
                    let diff = absdiff(preblend_result, true_result);
                    //println!("{} -- {} {} = {}", alpha, preblend_result, true_result, diff);
                    max_diff = max(max_diff, diff);
                }

                //println!("{} {} max {}", src, dst, max_diff);
                assert!(max_diff <= 33);
                dst += 1;

            }
            src += 1;
        }
    }
} // end mod
