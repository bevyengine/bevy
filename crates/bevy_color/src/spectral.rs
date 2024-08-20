use crate::{Alpha, Color, LinearRgba, Luminance, Xyza};
use bevy_math::FloatExt;
use std::ops::Mul;

/// A color produced by monochromatic light (of a single wavelength)
///
/// Since not every color is a spectral color, (e.g. magenta, white)
/// this type can be converted to `Color`, but not the other way around.
#[derive(Copy, Clone, Debug)]
pub struct SpectralColor {
    /// Wavelength in nanometers
    pub wavelength: f32,

    /// Luminance in candelas per square meter
    pub luminance: f32,
}

impl SpectralColor {
    /// Monochromatic light in the 830 nm wavelength
    pub const INFRARED: Self = Self {
        wavelength: 830.0,
        luminance: 1.0,
    };

    /// Monochromatic light in the 700 nm wavelength
    pub const RED: Self = Self {
        wavelength: 700.0,
        luminance: 1.0,
    };

    /// Monochromatic light in the 600 nm wavelength
    pub const ORANGE: Self = Self {
        wavelength: 600.0,
        luminance: 1.0,
    };

    /// Monochromatic light in the 570 nm wavelength
    pub const YELLOW: Self = Self {
        wavelength: 570.0,
        luminance: 1.0,
    };

    /// Monochromatic light in the 540 nm wavelength
    pub const GREEN: Self = Self {
        wavelength: 540.0,
        luminance: 1.0,
    };

    /// Monochromatic light in the 510 nm wavelength
    pub const CYAN: Self = Self {
        wavelength: 510.0,
        luminance: 1.0,
    };

    /// Monochromatic light in the 460 nm wavelength
    pub const BLUE: Self = Self {
        wavelength: 460.0,
        luminance: 1.0,
    };

    /// Monochromatic light in the 400 nm wavelength
    pub const VIOLET: Self = Self {
        wavelength: 400.0,
        luminance: 1.0,
    };

    /// Monochromatic light in the 380 nm wavelength
    pub const ULTRAVIOLET: Self = Self {
        wavelength: 380.0,
        luminance: 1.0,
    };

    /// Monochromatic light in the 589 nm wavelength, typically produced by sodium vapor lamps
    pub const SODIUM_VAPOR: Self = Self {
        wavelength: 589.0,
        luminance: 1.0,
    };

    /// Create a new spectral color with the given wavelength and luminance
    pub const fn new(wavelength: f32, luminance: f32) -> Self {
        Self {
            wavelength,
            luminance,
        }
    }

    /// Convert the spectral color to a Linear Rgba color, using the CIE 1931 2-deg XYZ color matching functions
    pub fn to_linear(&self) -> LinearRgba {
        if self.wavelength < Self::CIE_1931_NM_CMF_LOOKUP_TABLE_START
            || self.wavelength >= Self::CIE_1931_NM_CMF_LOOKUP_TABLE_END
        {
            // If the wavelength is outside the range of the lookup table, return black
            return LinearRgba::BLACK;
        }

        let index = ((self.wavelength - Self::CIE_1931_NM_CMF_LOOKUP_TABLE_START)
            / Self::CIE_1931_NM_CMF_LOOKUP_TABLE_INCREMENT)
            .floor() as usize;

        let lerp = (self.wavelength - Self::CIE_1931_NM_CMF_LOOKUP_TABLE_START)
            % Self::CIE_1931_NM_CMF_LOOKUP_TABLE_INCREMENT
            / Self::CIE_1931_NM_CMF_LOOKUP_TABLE_INCREMENT;

        let row = Self::CIE_1931_XYZ_CMF_LOOKUP_TABLE[index];
        let next_row = Self::CIE_1931_XYZ_CMF_LOOKUP_TABLE[index + 1];

        let x = row[0].lerp(next_row[0], lerp);
        let y = row[1].lerp(next_row[1], lerp);
        let z = row[2].lerp(next_row[2], lerp);

        let xyza = Xyza::new(x, y, z, 1.0);

        let mut linear = Color::from(xyza).to_linear();

        // Clamp negative values to zero
        linear.red = linear.red.max(0.0);
        linear.green = linear.green.max(0.0);
        linear.blue = linear.blue.max(0.0);

        // Apply luminance scaling, without clamping to white
        (linear * self.luminance).with_alpha(1.0)
    }

    /// Returns a new spectral color with the given wavelength
    pub fn with_wavelength(&self, wavelength: f32) -> Self {
        Self {
            wavelength,
            ..*self
        }
    }

    /// The wavelength where the look up table starts
    const CIE_1931_NM_CMF_LOOKUP_TABLE_START: f32 = 355.0;

    /// The wavelength where the look up table ends
    const CIE_1931_NM_CMF_LOOKUP_TABLE_END: f32 = 835.0;

    /// The increment between each row in the look up table
    const CIE_1931_NM_CMF_LOOKUP_TABLE_INCREMENT: f32 = 5.0;

    /// CIE 1931 2-deg, XYZ color matching functions, in lookup table form.
    /// Each row is a 5nm step from 360nm to 830nm (inclusive), with two
    /// rows of sentinel values, at the start and end of the table.
    /// (For interpolation to zero.)
    ///
    /// Source: <http://cvrl.ioo.ucl.ac.uk/plotcmfs.php>
    #[allow(clippy::excessive_precision)]
    const CIE_1931_XYZ_CMF_LOOKUP_TABLE: [[f32; 3]; 97] = [
        [0.000000000000, 0.000000000000, 0.000000000000], // Sentinel value
        [0.000129900000, 0.000003917000, 0.000606100000], // 360 nm
        [0.000232100000, 0.000006965000, 0.001086000000], // 365 nm
        [0.000414900000, 0.000012390000, 0.001946000000], // 370 nm
        [0.000741600000, 0.000022020000, 0.003486000000], // 375 nm
        [0.001368000000, 0.000039000000, 0.006450001000], // 380 nm
        [0.002236000000, 0.000064000000, 0.010549990000], // 385 nm
        [0.004243000000, 0.000120000000, 0.020050010000], // 390 nm
        [0.007650000000, 0.000217000000, 0.036210000000], // 395 nm
        [0.014310000000, 0.000396000000, 0.067850010000], // 400 nm
        [0.023190000000, 0.000640000000, 0.110200000000], // 405 nm
        [0.043510000000, 0.001210000000, 0.207400000000], // 410 nm
        [0.077630000000, 0.002180000000, 0.371300000000], // 415 nm
        [0.134380000000, 0.004000000000, 0.645600000000], // 420 nm
        [0.214770000000, 0.007300000000, 1.039050100000], // 425 nm
        [0.283900000000, 0.011600000000, 1.385600000000], // 430 nm
        [0.328500000000, 0.016840000000, 1.622960000000], // 435 nm
        [0.348280000000, 0.023000000000, 1.747060000000], // 440 nm
        [0.348060000000, 0.029800000000, 1.782600000000], // 445 nm
        [0.336200000000, 0.038000000000, 1.772110000000], // 450 nm
        [0.318700000000, 0.048000000000, 1.744100000000], // 455 nm
        [0.290800000000, 0.060000000000, 1.669200000000], // 460 nm
        [0.251100000000, 0.073900000000, 1.528100000000], // 465 nm
        [0.195360000000, 0.090980000000, 1.287640000000], // 470 nm
        [0.142100000000, 0.112600000000, 1.041900000000], // 475 nm
        [0.095640000000, 0.139020000000, 0.812950100000], // 480 nm
        [0.057950010000, 0.169300000000, 0.616200000000], // 485 nm
        [0.032010000000, 0.208020000000, 0.465180000000], // 490 nm
        [0.014700000000, 0.258600000000, 0.353300000000], // 495 nm
        [0.004900000000, 0.323000000000, 0.272000000000], // 500 nm
        [0.002400000000, 0.407300000000, 0.212300000000], // 505 nm
        [0.009300000000, 0.503000000000, 0.158200000000], // 510 nm
        [0.029100000000, 0.608200000000, 0.111700000000], // 515 nm
        [0.063270000000, 0.710000000000, 0.078249990000], // 520 nm
        [0.109600000000, 0.793200000000, 0.057250010000], // 525 nm
        [0.165500000000, 0.862000000000, 0.042160000000], // 530 nm
        [0.225749900000, 0.914850100000, 0.029840000000], // 535 nm
        [0.290400000000, 0.954000000000, 0.020300000000], // 540 nm
        [0.359700000000, 0.980300000000, 0.013400000000], // 545 nm
        [0.433449900000, 0.994950100000, 0.008749999000], // 550 nm
        [0.512050100000, 1.000000000000, 0.005749999000], // 555 nm
        [0.594500000000, 0.995000000000, 0.003900000000], // 560 nm
        [0.678400000000, 0.978600000000, 0.002749999000], // 565 nm
        [0.762100000000, 0.952000000000, 0.002100000000], // 570 nm
        [0.842500000000, 0.915400000000, 0.001800000000], // 575 nm
        [0.916300000000, 0.870000000000, 0.001650001000], // 580 nm
        [0.978600000000, 0.816300000000, 0.001400000000], // 585 nm
        [1.026300000000, 0.757000000000, 0.001100000000], // 590 nm
        [1.056700000000, 0.694900000000, 0.001000000000], // 595 nm
        [1.062200000000, 0.631000000000, 0.000800000000], // 600 nm
        [1.045600000000, 0.566800000000, 0.000600000000], // 605 nm
        [1.002600000000, 0.503000000000, 0.000340000000], // 610 nm
        [0.938400000000, 0.441200000000, 0.000240000000], // 615 nm
        [0.854449900000, 0.381000000000, 0.000190000000], // 620 nm
        [0.751400000000, 0.321000000000, 0.000100000000], // 625 nm
        [0.642400000000, 0.265000000000, 0.000049999990], // 630 nm
        [0.541900000000, 0.217000000000, 0.000030000000], // 635 nm
        [0.447900000000, 0.175000000000, 0.000020000000], // 640 nm
        [0.360800000000, 0.138200000000, 0.000010000000], // 645 nm
        [0.283500000000, 0.107000000000, 0.000000000000], // 650 nm
        [0.218700000000, 0.081600000000, 0.000000000000], // 655 nm
        [0.164900000000, 0.061000000000, 0.000000000000], // 660 nm
        [0.121200000000, 0.044580000000, 0.000000000000], // 665 nm
        [0.087400000000, 0.032000000000, 0.000000000000], // 670 nm
        [0.063600000000, 0.023200000000, 0.000000000000], // 675 nm
        [0.046770000000, 0.017000000000, 0.000000000000], // 680 nm
        [0.032900000000, 0.011920000000, 0.000000000000], // 685 nm
        [0.022700000000, 0.008210000000, 0.000000000000], // 690 nm
        [0.015840000000, 0.005723000000, 0.000000000000], // 695 nm
        [0.011359160000, 0.004102000000, 0.000000000000], // 700 nm
        [0.008110916000, 0.002929000000, 0.000000000000], // 705 nm
        [0.005790346000, 0.002091000000, 0.000000000000], // 710 nm
        [0.004109457000, 0.001484000000, 0.000000000000], // 715 nm
        [0.002899327000, 0.001047000000, 0.000000000000], // 720 nm
        [0.002049190000, 0.000740000000, 0.000000000000], // 725 nm
        [0.001439971000, 0.000520000000, 0.000000000000], // 730 nm
        [0.000999949300, 0.000361100000, 0.000000000000], // 735 nm
        [0.000690078600, 0.000249200000, 0.000000000000], // 740 nm
        [0.000476021300, 0.000171900000, 0.000000000000], // 745 nm
        [0.000332301100, 0.000120000000, 0.000000000000], // 750 nm
        [0.000234826100, 0.000084800000, 0.000000000000], // 755 nm
        [0.000166150500, 0.000060000000, 0.000000000000], // 760 nm
        [0.000117413000, 0.000042400000, 0.000000000000], // 765 nm
        [0.000083075270, 0.000030000000, 0.000000000000], // 770 nm
        [0.000058706520, 0.000021200000, 0.000000000000], // 775 nm
        [0.000041509940, 0.000014990000, 0.000000000000], // 780 nm
        [0.000029353260, 0.000010600000, 0.000000000000], // 785 nm
        [0.000020673830, 0.000007465700, 0.000000000000], // 790 nm
        [0.000014559770, 0.000005257800, 0.000000000000], // 795 nm
        [0.000010253980, 0.000003702900, 0.000000000000], // 800 nm
        [0.000007221456, 0.000002607800, 0.000000000000], // 805 nm
        [0.000005085868, 0.000001836600, 0.000000000000], // 810 nm
        [0.000003581652, 0.000001293400, 0.000000000000], // 815 nm
        [0.000002522525, 0.000000910930, 0.000000000000], // 820 nm
        [0.000001776509, 0.000000641530, 0.000000000000], // 825 nm
        [0.000001251141, 0.000000451810, 0.000000000000], // 830 nm
        [0.000000000000, 0.000000000000, 0.000000000000], // Sentinel value
    ];
}

impl Luminance for SpectralColor {
    fn luminance(&self) -> f32 {
        self.luminance
    }

    fn with_luminance(&self, value: f32) -> Self {
        Self {
            luminance: value,
            ..*self
        }
    }

    fn darker(&self, amount: f32) -> Self {
        self.with_luminance((self.luminance - amount).max(0.0))
    }

    fn lighter(&self, amount: f32) -> Self {
        self.with_luminance(self.luminance + amount)
    }
}

impl Mul<f32> for SpectralColor {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self {
            luminance: self.luminance * rhs,
            ..self
        }
    }
}
