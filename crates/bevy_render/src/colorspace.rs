// sRGB
//==================================================================================================
pub trait SrgbColorSpace {
    fn linear_to_nonlinear_srgb(self) -> Self;
    fn nonlinear_to_linear_srgb(self) -> Self;
}

//source: https://entropymine.com/imageworsener/srgbformula/
impl SrgbColorSpace for f32 {
    fn linear_to_nonlinear_srgb(self) -> f32 {
        if self <= 0.0 {
            return self;
        }

        if self <= 0.0031308 {
            self * 12.92 // linear falloff in dark values
        } else {
            (1.055 * self.powf(1.0 / 2.4)) - 0.055 //gamma curve in other area
        }
    }

    fn nonlinear_to_linear_srgb(self) -> f32 {
        if self <= 0.0 {
            return self;
        }
        if self <= 0.04045 {
            self / 12.92 // linear falloff in dark values
        } else {
            ((self + 0.055) / 1.055).powf(2.4) //gamma curve in other area
        }
    }
}

#[test]
fn test_srgb_full_roundtrip() {
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
//==================================================================================================
