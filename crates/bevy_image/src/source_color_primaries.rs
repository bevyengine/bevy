use bevy_color::RgbPrimaries;
#[cfg(not(feature = "bevy_reflect"))]
use bevy_reflect::TypePath;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use serde::{Deserialize, Serialize};

/// The color primaries that an [`Image`](crate::Image)'s RGB data is expressed in.
///
/// An image's color primaries are the exact red, green and blue that its RGB values
/// refer to. The white point is the color that equal amounts of all three produce.
/// Together they set the image's gamut, the range of colors it can express. Two images
/// with identical pixel values but different primaries show different colors.
/// [`RgbPrimaries`] documents the chromaticity coordinates behind each named set here.
///
/// This is metadata only. It records the gamut the pixel values were authored in.
/// Setting it does not convert the pixel data.
///
/// Loaders resolve the stamped value in this order:
/// 1. An explicit `source_color_primaries` loader setting, for example on
///    [`ImageLoaderSettings`](crate::ImageLoaderSettings).
/// 2. Color-primary metadata in the file. The KTX2, PNG, Radiance HDR and EXR loaders
///    read it.
/// 3. The [`SourceColorPrimaries::Bt709`] default, because most assets use the Rec. 709
///    primaries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Default, Debug, Clone, PartialEq, Hash)
)]
#[cfg_attr(not(feature = "bevy_reflect"), derive(TypePath))]
pub enum SourceColorPrimaries {
    /// The ITU-R BT.709 primaries with a D65 white point. sRGB uses the same primaries.
    /// See [`RgbPrimaries::BT709`] for the chromaticities and the standards link.
    #[default]
    Bt709,
    /// The ITU-R BT.2020 wide-gamut primaries with a D65 white point, also known as Rec. 2020.
    /// See [`RgbPrimaries::BT2020`] for the chromaticities and the standards link.
    Bt2020,
    /// The Display P3 primaries, which are the DCI-P3 primaries with a D65 white point.
    /// See [`RgbPrimaries::DISPLAY_P3`] for the chromaticities and the standards link.
    DisplayP3,
}

impl SourceColorPrimaries {
    /// Every supported primary set.
    const ALL: [Self; 3] = [Self::Bt709, Self::Bt2020, Self::DisplayP3];

    /// The per-coordinate tolerance used by [`SourceColorPrimaries::from_chromaticities`].
    ///
    /// A file matches a primary set when every coordinate is within this distance of the
    /// set's value. Files write primaries with three or four decimal places, so `2e-3`
    /// absorbs that rounding. The supported sets all differ by at least `0.09` in some
    /// coordinate, so a file can never match two sets.
    const CHROMATICITY_MATCH_TOLERANCE: f32 = 2e-3;

    /// Resolves the primaries to stamp on an [`Image`](crate::Image).
    ///
    /// An explicit loader setting wins and skips the file read, then file metadata,
    /// then the [`SourceColorPrimaries::Bt709`] default.
    #[cfg(any(feature = "exr", feature = "hdr", feature = "ktx2", feature = "png"))]
    pub(crate) fn resolve(
        setting: Option<Self>,
        file_metadata: impl FnOnce() -> Option<Self>,
    ) -> Self {
        setting.or_else(file_metadata).unwrap_or_default()
    }

    /// Returns the [`RgbPrimaries`] chromaticities of this primary set, for use with
    /// [`RgbPrimaries::matrix_to`].
    pub const fn to_rgb_primaries(self) -> RgbPrimaries {
        match self {
            SourceColorPrimaries::Bt709 => RgbPrimaries::BT709,
            SourceColorPrimaries::Bt2020 => RgbPrimaries::BT2020,
            SourceColorPrimaries::DisplayP3 => RgbPrimaries::DISPLAY_P3,
        }
    }

    /// Matches CIE 1931 xy chromaticities against the supported primary sets.
    ///
    /// Returns the matching set when every chromaticity is within a small tolerance of
    /// it. Returns `None` when no set matches.
    pub fn from_chromaticities(primaries: RgbPrimaries) -> Option<Self> {
        Self::ALL.into_iter().find(|candidate| {
            let reference = candidate.to_rgb_primaries();
            [
                (primaries.red, reference.red),
                (primaries.green, reference.green),
                (primaries.blue, reference.blue),
                (primaries.white, reference.white),
            ]
            .into_iter()
            .all(|(actual, expected)| {
                (actual.x - expected.x).abs() <= Self::CHROMATICITY_MATCH_TOLERANCE
                    && (actual.y - expected.y).abs() <= Self::CHROMATICITY_MATCH_TOLERANCE
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_color::Chromaticity;

    #[test]
    fn from_chromaticities_matches_within_tolerance() {
        for source in SourceColorPrimaries::ALL {
            let reference = source.to_rgb_primaries();
            assert_eq!(
                SourceColorPrimaries::from_chromaticities(reference),
                Some(source)
            );
            let nudge = SourceColorPrimaries::CHROMATICITY_MATCH_TOLERANCE * 0.5;
            assert_eq!(
                SourceColorPrimaries::from_chromaticities(RgbPrimaries {
                    red: Chromaticity::new(reference.red.x + nudge, reference.red.y - nudge),
                    ..reference
                }),
                Some(source)
            );
        }
    }

    #[test]
    fn from_chromaticities_rejects_unknown_primaries() {
        // ACEScg primaries are a valid file value but not a supported variant.
        assert_eq!(
            SourceColorPrimaries::from_chromaticities(RgbPrimaries::ACES_CG),
            None
        );
    }
}
