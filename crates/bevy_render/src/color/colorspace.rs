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
        } else if red > green && red > blue {
            60.0 * (green - blue) / chroma
        } else if green > red && green > blue {
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
}
