//! CIE 1931 chromaticity coordinates and RGB primary sets, used to convert linear RGB
//! values from one color space to another.
//!
//! Matrices are derived from chromaticity coordinates with the
//! [Lindbloom method](http://www.brucelindbloom.com/index.html?Eqn_RGB_XYZ_Matrix.html).

use crate::Xyza;
use bevy_math::{DMat3, DVec3, Mat3};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

/// A position in the [CIE 1931 xy chromaticity diagram](https://en.wikipedia.org/wiki/CIE_1931_color_space),
/// describing a color's hue and saturation independently of its luminance.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Clone, PartialEq, Default)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct Chromaticity {
    /// The x chromaticity coordinate. Physical colors are in `[0.0, 0.8]`.
    pub x: f32,
    /// The y chromaticity coordinate. Physical colors are in `(0.0, 0.9]`.
    pub y: f32,
}

impl Chromaticity {
    /// Construct a new [`Chromaticity`] from CIE 1931 xy coordinates.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// The [CIE Standard Illuminant D65](https://registry.color.org/rgb-registry/srgb#:~:text=White%20point%20chromaticity)
    /// white point, as specified by ITU-R BT.709 / BT.2020 and the sRGB standard.
    pub const D65: Self = Self::new(0.3127, 0.3290);

    /// The [ACES white point](https://docs.acescentral.com/white-point/), about
    /// CIE Standard Illuminant D60.
    pub const D60: Self = Self::new(0.32168, 0.33767);

    /// Convert this chromaticity and a luminance to a CIE 1931 color space.
    ///
    /// A luminance of `1.0` is the reference white. Alpha is set to `1.0`.
    pub const fn to_xyza(self, luminance: f32) -> Xyza {
        let xyz = self.to_dxyz();
        Xyza::new(
            (xyz.x * luminance as f64) as f32,
            luminance,
            (xyz.z * luminance as f64) as f32,
            1.0,
        )
    }

    /// The CIE 1931 XYZ tristimulus value of this chromaticity at luminance `1.0`,
    /// in `f64` precision for matrix derivation.
    const fn to_dxyz(self) -> DVec3 {
        let (x, y) = (self.x as f64, self.y as f64);
        DVec3::new(x / y, 1.0, (1.0 - x - y) / y)
    }
}

impl Default for Chromaticity {
    fn default() -> Self {
        Self::D65
    }
}

/// A set of RGB primaries and a white point, given as [`Chromaticity`] coordinates.
///
/// Together they define what a linear RGB color looks like, aside from overall brightness.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Clone, PartialEq, Default)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct RgbPrimaries {
    /// The chromaticity of the red primary.
    pub red: Chromaticity,
    /// The chromaticity of the green primary.
    pub green: Chromaticity,
    /// The chromaticity of the blue primary.
    pub blue: Chromaticity,
    /// The chromaticity of the white point (the color produced by `(1, 1, 1)`).
    pub white: Chromaticity,
}

impl RgbPrimaries {
    /// The [ITU-R BT.709](https://registry.color.org/rgb-registry/bt709) primaries with a D65
    /// white point.
    ///
    /// sRGB and [`LinearRgba`](crate::LinearRgba) use these primaries.
    pub const BT709: Self = Self {
        red: Chromaticity::new(0.640, 0.330),
        green: Chromaticity::new(0.300, 0.600),
        blue: Chromaticity::new(0.150, 0.060),
        white: Chromaticity::D65,
    };

    /// The [ITU-R BT.2020](https://registry.color.org/rgb-registry/bt2020) (Rec. 2020)
    /// wide-gamut primaries with a D65 white point.
    pub const BT2020: Self = Self {
        red: Chromaticity::new(0.708, 0.292),
        green: Chromaticity::new(0.170, 0.797),
        blue: Chromaticity::new(0.131, 0.046),
        white: Chromaticity::D65,
    };

    /// The [Display P3](https://registry.color.org/rgb-registry/displayp3) primaries:
    /// the DCI-P3 primaries with a D65 white point.
    pub const DISPLAY_P3: Self = Self {
        red: Chromaticity::new(0.680, 0.320),
        green: Chromaticity::new(0.265, 0.690),
        blue: Chromaticity::new(0.150, 0.060),
        white: Chromaticity::D65,
    };

    /// The [ACEScg](https://docs.acescentral.com/encodings/acescg) (AP1) primaries
    /// with the ACES white point, used for scene-linear rendering.
    pub const ACES_CG: Self = Self {
        red: Chromaticity::new(0.713, 0.293),
        green: Chromaticity::new(0.165, 0.830),
        blue: Chromaticity::new(0.128, 0.044),
        white: Chromaticity::D60,
    };

    /// Returns a matrix that converts linear RGB colors from these primaries to `dst`
    /// primaries.
    ///
    /// White points are not adapted. Converting between sets with different whites, such as
    /// [`RgbPrimaries::ACES_CG`] and a D65 set, leaves a small shift in white.
    /// The matrix is computed in `f64` and rounded to `f32`.
    pub fn matrix_to(self, dst: Self) -> Mat3 {
        (dst.rgb_to_xyz_dmat3().inverse() * self.rgb_to_xyz_dmat3()).as_mat3()
    }

    /// Derive the RGB to XYZ matrix in `f64` precision. The matrix maps `(1, 1, 1)`
    /// to the XYZ value of [`Self::white`] at luminance `1.0`.
    fn rgb_to_xyz_dmat3(&self) -> DMat3 {
        let primaries = DMat3::from_cols(
            self.red.to_dxyz(),
            self.green.to_dxyz(),
            self.blue.to_dxyz(),
        );
        // Scale each column so that (1, 1, 1) maps to the white point at Y = 1.
        let scale = primaries.inverse() * self.white.to_dxyz();
        primaries * DMat3::from_diagonal(scale)
    }
}

impl Default for RgbPrimaries {
    fn default() -> Self {
        Self::BT709
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_approx_eq, assert_mat3_approx_eq};
    use bevy_math::Vec3;

    #[test]
    fn bt709_to_bt2020_matches_bt2087() {
        // Published ITU-R BT.2087-0 (table in section 2.2.1) Rec.709 to Rec.2020 matrix.
        let expected = Mat3::from_cols_array_2d(&[
            [0.6274, 0.0691, 0.0164],
            [0.3293, 0.9195, 0.0880],
            [0.0433, 0.0114, 0.8956],
        ]);
        let m = RgbPrimaries::BT709.matrix_to(RgbPrimaries::BT2020);
        assert_mat3_approx_eq(m, expected, 1e-4);
    }

    #[test]
    fn rgb_to_rgb_round_trip_is_identity() {
        let sets = [
            RgbPrimaries::BT709,
            RgbPrimaries::BT2020,
            RgbPrimaries::DISPLAY_P3,
            RgbPrimaries::ACES_CG,
        ];
        for src in sets {
            for dst in sets {
                let round_trip = dst.matrix_to(src) * src.matrix_to(dst);
                assert_mat3_approx_eq(round_trip, Mat3::IDENTITY, 1e-6);
            }
        }
    }

    #[test]
    fn white_maps_to_white_point() {
        for primaries in [
            RgbPrimaries::BT709,
            RgbPrimaries::BT2020,
            RgbPrimaries::DISPLAY_P3,
            RgbPrimaries::ACES_CG,
        ] {
            let white_xyz = primaries.rgb_to_xyz_dmat3().as_mat3() * Vec3::ONE;
            let expected = primaries.white.to_xyza(1.0);
            assert_approx_eq!(white_xyz.x, expected.x, 1e-6);
            assert_approx_eq!(white_xyz.y, expected.y, 1e-6);
            assert_approx_eq!(white_xyz.z, expected.z, 1e-6);
        }
    }

    #[test]
    fn bt709_matrix_matches_crate_srgb_matrix() {
        // This crate's sRGB matrices use Lindbloom's D65, XYZ 0.95047, 1.0, 1.08883 from
        // ASTM E308. White from the BT.709 chromaticity 0.3127, 0.3290 differs by about
        // 2e-4, so this check is only approximate.
        let expected = Mat3::from_cols_array_2d(&[
            [0.4124564, 0.2126729, 0.0193339],
            [0.3575761, 0.7151522, 0.119192],
            [0.1804375, 0.072175, 0.9503041],
        ]);
        let m = RgbPrimaries::BT709.rgb_to_xyz_dmat3().as_mat3();
        assert_mat3_approx_eq(m, expected, 5e-4);
    }

    #[test]
    fn chromaticity_to_xyza() {
        let xyz = Chromaticity::D65.to_xyza(1.0);
        assert_approx_eq!(xyz.x, 0.9504559, 1e-6);
        assert_approx_eq!(xyz.y, 1.0, 1e-6);
        assert_approx_eq!(xyz.z, 1.0890578, 1e-6);
        assert_approx_eq!(xyz.alpha, 1.0, 1e-6);

        let xyz = Chromaticity::D65.to_xyza(2.0);
        assert_approx_eq!(xyz.y, 2.0, 1e-6);
        assert_approx_eq!(xyz.x, 2.0 * 0.9504559, 1e-5);
    }
}
