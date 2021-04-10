use super::*;

trait SrgbColorSpace {
    fn linear_to_nonlinear_srgb(self) -> Self;
    fn nonlinear_to_linear_srgb(self) -> Self;
}

// source: https://entropymine.com/imageworsener/srgbformula/
impl SrgbColorSpace for f32 {
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

impl From<LinSrgba> for Srgba {
    fn from(linsrgba: LinSrgba) -> Self {
        Srgba {
            r: linsrgba.r.linear_to_nonlinear_srgb(),
            g: linsrgba.g.linear_to_nonlinear_srgb(),
            b: linsrgba.b.linear_to_nonlinear_srgb(),
            a: linsrgba.a,
        }
    }
}

impl From<LinSrgba> for Hsla {
    fn from(linsrgba: LinSrgba) -> Self {
        Srgba::from(linsrgba).into()
    }
}

impl From<Hsla> for Srgba {
    fn from(hsla: Hsla) -> Self {
        // https://en.wikipedia.org/wiki/HSL_and_HSV#HSL_to_RGB
        let chroma = (1.0 - (2.0 * hsla.l - 1.0).abs()) * hsla.s;
        let hue_prime = hsla.h / 60.0;
        let largest_component = chroma * (1.0 - (hue_prime % 2.0 - 1.0).abs());
        let (r, g, b) = if hue_prime < 1.0 {
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
        let lightness_match = hsla.l - chroma / 2.0;

        Srgba::with_alpha(
            r + lightness_match,
            g + lightness_match,
            b + lightness_match,
            hsla.a,
        )
    }
}

impl From<Hsla> for LinSrgba {
    fn from(hsla: Hsla) -> Self {
        Srgba::from(hsla).into()
    }
}

impl From<Srgba> for LinSrgba {
    fn from(srgba: Srgba) -> Self {
        LinSrgba {
            r: srgba.r.nonlinear_to_linear_srgb(),
            g: srgba.g.nonlinear_to_linear_srgb(),
            b: srgba.b.nonlinear_to_linear_srgb(),
            a: srgba.a,
        }
    }
}

impl From<Srgba> for Hsla {
    fn from(srgba: Srgba) -> Self {
        // https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB
        let x_max = srgba.r.max(srgba.g.max(srgba.b));
        let x_min = srgba.r.min(srgba.g.min(srgba.b));
        let chroma = x_max - x_min;
        let lightness = (x_max + x_min) / 2.0;
        let hue = if chroma == 0.0 {
            0.0
        } else if srgba.r > srgba.g && srgba.r > srgba.b {
            60.0 * (srgba.g - srgba.b) / chroma
        } else if srgba.g > srgba.r && srgba.g > srgba.b {
            60.0 * (2.0 + (srgba.b - srgba.r) / chroma)
        } else {
            60.0 * (4.0 + (srgba.r - srgba.g) / chroma)
        };
        let hue = if hue < 0.0 { 360.0 + hue } else { hue };
        let saturation = if lightness <= 0.0 || lightness >= 1.0 {
            0.0
        } else {
            (x_max - lightness) / lightness.min(1.0 - lightness)
        };

        Hsla::with_alpha(hue, saturation, lightness, srgba.a)
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
        let color = Srgba::from(Hsla::new(hue, saturation, lightness));
        assert_eq!((color.r * 100.0).round() as u32, 0);
        assert_eq!((color.g * 100.0).round() as u32, 0);
        assert_eq!((color.b * 100.0).round() as u32, 0);

        // white
        let (hue, saturation, lightness) = (0.0, 0.0, 1.0);
        let color = Srgba::from(Hsla::new(hue, saturation, lightness));
        assert_eq!((color.r * 100.0).round() as u32, 100);
        assert_eq!((color.g * 100.0).round() as u32, 100);
        assert_eq!((color.b * 100.0).round() as u32, 100);

        let (hue, saturation, lightness) = (300.0, 0.5, 0.5);
        let color = Srgba::from(Hsla::new(hue, saturation, lightness));
        assert_eq!((color.r * 100.0).round() as u32, 75);
        assert_eq!((color.g * 100.0).round() as u32, 25);
        assert_eq!((color.b * 100.0).round() as u32, 75);

        // a red
        let (hue, saturation, lightness) = (283.7, 0.775, 0.543);
        let color = Srgba::from(Hsla::new(hue, saturation, lightness));
        assert_eq!((color.r * 100.0).round() as u32, 70);
        assert_eq!((color.g * 100.0).round() as u32, 19);
        assert_eq!((color.b * 100.0).round() as u32, 90);

        // a green
        let (hue, saturation, lightness) = (162.4, 0.779, 0.447);
        let color = Srgba::from(Hsla::new(hue, saturation, lightness));
        assert_eq!((color.r * 100.0).round() as u32, 10);
        assert_eq!((color.g * 100.0).round() as u32, 80);
        assert_eq!((color.b * 100.0).round() as u32, 59);

        // a blue
        let (hue, saturation, lightness) = (251.1, 0.832, 0.511);
        let color = Srgba::from(Hsla::new(hue, saturation, lightness));
        assert_eq!((color.r * 100.0).round() as u32, 25);
        assert_eq!((color.g * 100.0).round() as u32, 10);
        assert_eq!((color.b * 100.0).round() as u32, 92);
    }

    #[test]
    fn srgb_to_hsl() {
        // "truth" from https://en.wikipedia.org/wiki/HSL_and_HSV#Examples

        // black
        let color = Hsla::from(Srgba::new(0.0, 0.0, 0.0));
        assert_eq!(color.h.round() as u32, 0);
        assert_eq!((color.s * 100.0).round() as u32, 0);
        assert_eq!((color.l * 100.0).round() as u32, 0);

        // white
        let color = Hsla::from(Srgba::new(1.0, 1.0, 1.0));
        assert_eq!(color.h.round() as u32, 0);
        assert_eq!((color.s * 100.0).round() as u32, 0);
        assert_eq!((color.l * 100.0).round() as u32, 100);

        let color = Hsla::from(Srgba::new(0.75, 0.25, 0.75));
        assert_eq!(color.h.round() as u32, 300);
        assert_eq!((color.s * 100.0).round() as u32, 50);
        assert_eq!((color.l * 100.0).round() as u32, 50);

        // a red
        let color = Hsla::from(Srgba::new(0.704, 0.187, 0.897));
        assert_eq!(color.h.round() as u32, 284);
        assert_eq!((color.s * 100.0).round() as u32, 78);
        assert_eq!((color.l * 100.0).round() as u32, 54);

        // a green
        let color = Hsla::from(Srgba::new(0.099, 0.795, 0.591));
        assert_eq!(color.h.round() as u32, 162);
        assert_eq!((color.s * 100.0).round() as u32, 78);
        assert_eq!((color.l * 100.0).round() as u32, 45);

        // a blue
        let color = Hsla::from(Srgba::new(0.255, 0.104, 0.918));
        assert_eq!(color.h.round() as u32, 251);
        assert_eq!((color.s * 100.0).round() as u32, 83);
        assert_eq!((color.l * 100.0).round() as u32, 51);
    }
}
