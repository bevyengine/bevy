pub trait SrgbColorSpace {
    fn linear_to_nonlinear_srgb(self) -> Self;
    fn nonlinear_to_linear_srgb(self) -> Self;
}

// source: https://entropymine.com/imageworsener/srgbformula/
impl SrgbColorSpace for f32 {
    #[inline]
    fn linear_to_nonlinear_srgb(self) -> f32 {
        if self <= 0.0 {
            return self;
        }

        if self <= 0.0031308 {
            self * 12.92 // linear falloff in dark values
        } else {
            (1.055 * self.powf(1.0 / 2.4)) - 0.055 // gamma curve in other area
        }
    }

    #[inline]
    fn nonlinear_to_linear_srgb(self) -> f32 {
        if self <= 0.0 {
            return self;
        }
        if self <= 0.04045 {
            self / 12.92 // linear falloff in dark values
        } else {
            ((self + 0.055) / 1.055).powf(2.4) // gamma curve in other area
        }
    }
}

impl SrgbColorSpace for u8 {
    #[inline]
    fn linear_to_nonlinear_srgb(self) -> Self {
        ((self as f32 / u8::MAX as f32).linear_to_nonlinear_srgb() * u8::MAX as f32) as u8
    }

    #[inline]
    fn nonlinear_to_linear_srgb(self) -> Self {
        ((self as f32 / u8::MAX as f32).nonlinear_to_linear_srgb() * u8::MAX as f32) as u8
    }
}

pub struct HslRepresentation;
impl HslRepresentation {
    /// converts a color in HLS space to sRGB space
    #[inline]
    pub fn hsl_to_nonlinear_srgb(hue: f32, saturation: f32, lightness: f32) -> [f32; 3] {
        // https://en.wikipedia.org/wiki/HSL_and_HSV#HSL_to_RGB
        let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
        let hue_prime = hue / 60.0;
        let largest_component = chroma * (1.0 - (hue_prime % 2.0 - 1.0).abs());
        let (r_temp, g_temp, b_temp) = if hue_prime < 1.0 {
            (chroma, largest_component, 0.0)
        } else if hue_prime < 2.0 {
            (largest_component, chroma, 0.0)
        } else if hue_prime < 3.0 {
            (0.0, chroma, largest_component)
        } else if hue_prime < 4.0 {
            (0.0, largest_component, chroma)
        } else if hue_prime < 5.0 {
            (largest_component, 0.0, chroma)
        } else {
            (chroma, 0.0, largest_component)
        };
        let lightness_match = lightness - chroma / 2.0;

        [
            r_temp + lightness_match,
            g_temp + lightness_match,
            b_temp + lightness_match,
        ]
    }

    /// converts a color in sRGB space to HLS space
    #[inline]
    pub fn nonlinear_srgb_to_hsl([red, green, blue]: [f32; 3]) -> (f32, f32, f32) {
        // https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB
        let x_max = red.max(green.max(blue));
        let x_min = red.min(green.min(blue));
        let chroma = x_max - x_min;
        let lightness = (x_max + x_min) / 2.0;
        let hue = if chroma == 0.0 {
            0.0
        } else if red == x_max {
            60.0 * (green - blue) / chroma
        } else if green == x_max {
            60.0 * (2.0 + (blue - red) / chroma)
        } else {
            60.0 * (4.0 + (red - green) / chroma)
        };
        let hue = if hue < 0.0 { 360.0 + hue } else { hue };
        let saturation = if lightness <= 0.0 || lightness >= 1.0 {
            0.0
        } else {
            (x_max - lightness) / lightness.min(1.0 - lightness)
        };

        (hue, saturation, lightness)
    }
}

pub struct LchRepresentation;
impl LchRepresentation {
    // References available at http://brucelindbloom.com/ in the "Math" section

    // CIE Constants
    // http://brucelindbloom.com/index.html?LContinuity.html (16) (17)
    const CIE_EPSILON: f32 = 216.0 / 24389.0;
    const CIE_KAPPA: f32 = 24389.0 / 27.0;
    // D65 White Reference:
    // https://en.wikipedia.org/wiki/Illuminant_D65#Definition
    const D65_WHITE_X: f32 = 0.95047;
    const D65_WHITE_Y: f32 = 1.0;
    const D65_WHITE_Z: f32 = 1.08883;

    /// converts a color in LCH space to sRGB space
    #[inline]
    pub fn lch_to_nonlinear_srgb(lightness: f32, chroma: f32, hue: f32) -> [f32; 3] {
        let lightness = lightness * 100.0;
        let chroma = chroma * 100.0;

        // convert LCH to Lab
        // http://www.brucelindbloom.com/index.html?Eqn_LCH_to_Lab.html
        let l = lightness;
        let a = chroma * hue.to_radians().cos();
        let b = chroma * hue.to_radians().sin();

        // convert Lab to XYZ
        // http://www.brucelindbloom.com/index.html?Eqn_Lab_to_XYZ.html
        let fy = (l + 16.0) / 116.0;
        let fx = a / 500.0 + fy;
        let fz = fy - b / 200.0;
        let xr = {
            let fx3 = fx.powf(3.0);

            if fx3 > Self::CIE_EPSILON {
                fx3
            } else {
                (116.0 * fx - 16.0) / Self::CIE_KAPPA
            }
        };
        let yr = if l > Self::CIE_EPSILON * Self::CIE_KAPPA {
            ((l + 16.0) / 116.0).powf(3.0)
        } else {
            l / Self::CIE_KAPPA
        };
        let zr = {
            let fz3 = fz.powf(3.0);

            if fz3 > Self::CIE_EPSILON {
                fz3
            } else {
                (116.0 * fz - 16.0) / Self::CIE_KAPPA
            }
        };
        let x = xr * Self::D65_WHITE_X;
        let y = yr * Self::D65_WHITE_Y;
        let z = zr * Self::D65_WHITE_Z;

        // XYZ to sRGB
        // http://www.brucelindbloom.com/index.html?Eqn_XYZ_to_RGB.html
        // http://www.brucelindbloom.com/index.html?Eqn_RGB_XYZ_Matrix.html (sRGB, XYZ to RGB [M]-1)
        let red = x * 3.2404542 + y * -1.5371385 + z * -0.4985314;
        let green = x * -0.969266 + y * 1.8760108 + z * 0.041556;
        let blue = x * 0.0556434 + y * -0.2040259 + z * 1.0572252;

        [
            red.linear_to_nonlinear_srgb().clamp(0.0, 1.0),
            green.linear_to_nonlinear_srgb().clamp(0.0, 1.0),
            blue.linear_to_nonlinear_srgb().clamp(0.0, 1.0),
        ]
    }

    /// converts a color in sRGB space to LCH space
    #[inline]
    pub fn nonlinear_srgb_to_lch([red, green, blue]: [f32; 3]) -> (f32, f32, f32) {
        // RGB to XYZ
        // http://www.brucelindbloom.com/index.html?Eqn_RGB_to_XYZ.html
        let red = red.nonlinear_to_linear_srgb();
        let green = green.nonlinear_to_linear_srgb();
        let blue = blue.nonlinear_to_linear_srgb();

        // http://www.brucelindbloom.com/index.html?Eqn_RGB_XYZ_Matrix.html (sRGB, RGB to XYZ [M])
        let x = red * 0.4124564 + green * 0.3575761 + blue * 0.1804375;
        let y = red * 0.2126729 + green * 0.7151522 + blue * 0.072175;
        let z = red * 0.0193339 + green * 0.119192 + blue * 0.9503041;

        // XYZ to Lab
        // http://www.brucelindbloom.com/index.html?Eqn_XYZ_to_Lab.html
        let xr = x / Self::D65_WHITE_X;
        let yr = y / Self::D65_WHITE_Y;
        let zr = z / Self::D65_WHITE_Z;
        let fx = if xr > Self::CIE_EPSILON {
            xr.cbrt()
        } else {
            (Self::CIE_KAPPA * xr + 16.0) / 116.0
        };
        let fy = if yr > Self::CIE_EPSILON {
            yr.cbrt()
        } else {
            (Self::CIE_KAPPA * yr + 16.0) / 116.0
        };
        let fz = if yr > Self::CIE_EPSILON {
            zr.cbrt()
        } else {
            (Self::CIE_KAPPA * zr + 16.0) / 116.0
        };
        let l = 116.0 * fy - 16.0;
        let a = 500.0 * (fx - fy);
        let b = 200.0 * (fy - fz);

        // Lab to LCH
        // http://www.brucelindbloom.com/index.html?Eqn_Lab_to_LCH.html
        let c = (a.powf(2.0) + b.powf(2.0)).sqrt();
        let h = {
            let h = b.to_radians().atan2(a.to_radians()).to_degrees();

            if h < 0.0 {
                h + 360.0
            } else {
                h
            }
        };

        ((l / 100.0).clamp(0.0, 1.5), (c / 100.0).clamp(0.0, 1.5), h)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn srgb_linear_full_roundtrip() {
        let u8max: f32 = u8::max_value() as f32;
        for color in 0..u8::max_value() {
            let color01 = color as f32 / u8max;
            let color_roundtrip = color01
                .linear_to_nonlinear_srgb()
                .nonlinear_to_linear_srgb();
            // roundtrip is not perfect due to numeric precision, even with f64
            // so ensure the error is at least ready for u8 (where sRGB is used)
            assert_eq!(
                (color01 * u8max).round() as u8,
                (color_roundtrip * u8max).round() as u8
            );
        }
    }

    #[test]
    fn hsl_to_srgb() {
        // "truth" from https://en.wikipedia.org/wiki/HSL_and_HSV#Examples

        // black
        let (hue, saturation, lightness) = (0.0, 0.0, 0.0);
        let [r, g, b] = HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
        assert_eq!((r * 100.0).round() as u32, 0);
        assert_eq!((g * 100.0).round() as u32, 0);
        assert_eq!((b * 100.0).round() as u32, 0);

        // white
        let (hue, saturation, lightness) = (0.0, 0.0, 1.0);
        let [r, g, b] = HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
        assert_eq!((r * 100.0).round() as u32, 100);
        assert_eq!((g * 100.0).round() as u32, 100);
        assert_eq!((b * 100.0).round() as u32, 100);

        let (hue, saturation, lightness) = (300.0, 0.5, 0.5);
        let [r, g, b] = HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
        assert_eq!((r * 100.0).round() as u32, 75);
        assert_eq!((g * 100.0).round() as u32, 25);
        assert_eq!((b * 100.0).round() as u32, 75);

        // a red
        let (hue, saturation, lightness) = (283.7, 0.775, 0.543);
        let [r, g, b] = HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
        assert_eq!((r * 100.0).round() as u32, 70);
        assert_eq!((g * 100.0).round() as u32, 19);
        assert_eq!((b * 100.0).round() as u32, 90);

        // a green
        let (hue, saturation, lightness) = (162.4, 0.779, 0.447);
        let [r, g, b] = HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
        assert_eq!((r * 100.0).round() as u32, 10);
        assert_eq!((g * 100.0).round() as u32, 80);
        assert_eq!((b * 100.0).round() as u32, 59);

        // a blue
        let (hue, saturation, lightness) = (251.1, 0.832, 0.511);
        let [r, g, b] = HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
        assert_eq!((r * 100.0).round() as u32, 25);
        assert_eq!((g * 100.0).round() as u32, 10);
        assert_eq!((b * 100.0).round() as u32, 92);
    }

    #[test]
    fn srgb_to_hsl() {
        // "truth" from https://en.wikipedia.org/wiki/HSL_and_HSV#Examples

        // black
        let (hue, saturation, lightness) =
            HslRepresentation::nonlinear_srgb_to_hsl([0.0, 0.0, 0.0]);
        assert_eq!(hue.round() as u32, 0);
        assert_eq!((saturation * 100.0).round() as u32, 0);
        assert_eq!((lightness * 100.0).round() as u32, 0);

        // white
        let (hue, saturation, lightness) =
            HslRepresentation::nonlinear_srgb_to_hsl([1.0, 1.0, 1.0]);
        assert_eq!(hue.round() as u32, 0);
        assert_eq!((saturation * 100.0).round() as u32, 0);
        assert_eq!((lightness * 100.0).round() as u32, 100);

        let (hue, saturation, lightness) =
            HslRepresentation::nonlinear_srgb_to_hsl([0.75, 0.25, 0.75]);
        assert_eq!(hue.round() as u32, 300);
        assert_eq!((saturation * 100.0).round() as u32, 50);
        assert_eq!((lightness * 100.0).round() as u32, 50);

        // a red
        let (hue, saturation, lightness) =
            HslRepresentation::nonlinear_srgb_to_hsl([0.704, 0.187, 0.897]);
        assert_eq!(hue.round() as u32, 284);
        assert_eq!((saturation * 100.0).round() as u32, 78);
        assert_eq!((lightness * 100.0).round() as u32, 54);

        // a green
        let (hue, saturation, lightness) =
            HslRepresentation::nonlinear_srgb_to_hsl([0.099, 0.795, 0.591]);
        assert_eq!(hue.round() as u32, 162);
        assert_eq!((saturation * 100.0).round() as u32, 78);
        assert_eq!((lightness * 100.0).round() as u32, 45);

        // a blue
        let (hue, saturation, lightness) =
            HslRepresentation::nonlinear_srgb_to_hsl([0.255, 0.104, 0.918]);
        assert_eq!(hue.round() as u32, 251);
        assert_eq!((saturation * 100.0).round() as u32, 83);
        assert_eq!((lightness * 100.0).round() as u32, 51);
    }

    #[test]
    fn lch_to_srgb() {
        // "truth" from http://www.brucelindbloom.com/ColorCalculator.html

        // black
        let (lightness, chroma, hue) = (0.0, 0.0, 0.0);
        let [r, g, b] = LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);
        assert_eq!((r * 100.0).round() as u32, 0);
        assert_eq!((g * 100.0).round() as u32, 0);
        assert_eq!((b * 100.0).round() as u32, 0);

        // white
        let (lightness, chroma, hue) = (1.0, 0.0, 0.0);
        let [r, g, b] = LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);
        assert_eq!((r * 100.0).round() as u32, 100);
        assert_eq!((g * 100.0).round() as u32, 100);
        assert_eq!((b * 100.0).round() as u32, 100);

        let (lightness, chroma, hue) = (0.501236, 0.777514, 327.6608);
        let [r, g, b] = LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);
        assert_eq!((r * 100.0).round() as u32, 75);
        assert_eq!((g * 100.0).round() as u32, 25);
        assert_eq!((b * 100.0).round() as u32, 75);

        // a red
        let (lightness, chroma, hue) = (0.487122, 0.999531, 318.7684);
        let [r, g, b] = LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);
        assert_eq!((r * 100.0).round() as u32, 70);
        assert_eq!((g * 100.0).round() as u32, 19);
        assert_eq!((b * 100.0).round() as u32, 90);

        // a green
        let (lightness, chroma, hue) = (0.732929, 0.560925, 164.3216);
        let [r, g, b] = LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);
        assert_eq!((r * 100.0).round() as u32, 10);
        assert_eq!((g * 100.0).round() as u32, 80);
        assert_eq!((b * 100.0).round() as u32, 59);

        // a blue
        let (lightness, chroma, hue) = (0.335030, 1.176923, 306.7828);
        let [r, g, b] = LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);
        assert_eq!((r * 100.0).round() as u32, 25);
        assert_eq!((g * 100.0).round() as u32, 10);
        assert_eq!((b * 100.0).round() as u32, 92);
    }

    #[test]
    fn srgb_to_lch() {
        // "truth" from http://www.brucelindbloom.com/ColorCalculator.html

        // black
        let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch([0.0, 0.0, 0.0]);
        assert_eq!((lightness * 100.0).round() as u32, 0);
        assert_eq!((chroma * 100.0).round() as u32, 0);
        assert_eq!(hue.round() as u32, 0);

        // white
        let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch([1.0, 1.0, 1.0]);
        assert_eq!((lightness * 100.0).round() as u32, 100);
        assert_eq!((chroma * 100.0).round() as u32, 0);
        assert_eq!(hue.round() as u32, 0);

        let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch([0.75, 0.25, 0.75]);
        assert_eq!((lightness * 100.0).round() as u32, 50);
        assert_eq!((chroma * 100.0).round() as u32, 78);
        assert_eq!(hue.round() as u32, 328);

        // a red
        let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch([0.70, 0.19, 0.90]);
        assert_eq!((lightness * 100.0).round() as u32, 49);
        assert_eq!((chroma * 100.0).round() as u32, 100);
        assert_eq!(hue.round() as u32, 319);

        // a green
        let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch([0.10, 0.80, 0.59]);
        assert_eq!((lightness * 100.0).round() as u32, 73);
        assert_eq!((chroma * 100.0).round() as u32, 56);
        assert_eq!(hue.round() as u32, 164);

        // a blue
        let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch([0.25, 0.10, 0.92]);
        assert_eq!((lightness * 100.0).round() as u32, 34);
        assert_eq!((chroma * 100.0).round() as u32, 118);
        assert_eq!(hue.round() as u32, 307);
    }
}
