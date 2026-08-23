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
///
/// This is metadata only. Setting it does not convert the pixel data.
/// [`to_rgb_primaries`](Self::to_rgb_primaries) returns each set's chromaticities as
/// an [`RgbPrimaries`].
///
/// Loaders pick the value in this order:
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
    /// The ITU-R BT.709 primaries, [`RgbPrimaries::BT709`]. Also used by sRGB.
    #[default]
    Bt709,
    /// The ITU-R BT.2020 primaries, [`RgbPrimaries::BT2020`]. Also known as Rec. 2020.
    Bt2020,
    /// The Display P3 primaries, [`RgbPrimaries::DISPLAY_P3`].
    DisplayP3,
}

impl SourceColorPrimaries {
    /// Every supported primary set.
    const ALL: [Self; 3] = [Self::Bt709, Self::Bt2020, Self::DisplayP3];

    /// The chromaticity tolerance used by [`SourceColorPrimaries::from_chromaticities`].
    ///
    /// Chromaticities match a set when the absolute differences of all eight coordinates
    /// sum to less than this value. ffmpeg and libplacebo detect primaries the same way.
    /// The supported sets differ by at least `0.09` in some coordinate, so no input can
    /// match two sets.
    const CHROMATICITY_MATCH_TOLERANCE: f32 = 1e-3;

    /// Resolves the primaries to set on an [`Image`](crate::Image), following the order
    /// in the type docs.
    #[cfg(any(feature = "exr", feature = "hdr", feature = "ktx2", feature = "png"))]
    pub(crate) fn resolve(
        setting: Option<Self>,
        file_metadata: impl FnOnce() -> Option<Self>,
    ) -> Self {
        setting.or_else(file_metadata).unwrap_or_default()
    }

    /// Returns the [`RgbPrimaries`] chromaticities of this primary set.
    pub const fn to_rgb_primaries(self) -> RgbPrimaries {
        match self {
            SourceColorPrimaries::Bt709 => RgbPrimaries::BT709,
            SourceColorPrimaries::Bt2020 => RgbPrimaries::BT2020,
            SourceColorPrimaries::DisplayP3 => RgbPrimaries::DISPLAY_P3,
        }
    }

    /// Matches CIE 1931 xy chromaticities against the supported primary sets.
    ///
    /// Returns the matching set, or `None` when no set is close enough.
    pub fn from_chromaticities(primaries: RgbPrimaries) -> Option<Self> {
        Self::ALL.into_iter().find(|candidate| {
            let reference = candidate.to_rgb_primaries();
            let delta: f32 = [
                (primaries.red, reference.red),
                (primaries.green, reference.green),
                (primaries.blue, reference.blue),
                (primaries.white, reference.white),
            ]
            .into_iter()
            .map(|(actual, expected)| (actual.x - expected.x).abs() + (actual.y - expected.y).abs())
            .sum();
            delta < Self::CHROMATICITY_MATCH_TOLERANCE
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_color::Chromaticity;

    #[test]
    fn from_chromaticities_matches_three_decimal_values() {
        fn round3(c: Chromaticity) -> Chromaticity {
            Chromaticity::new(
                (c.x * 1000.0).round() / 1000.0,
                (c.y * 1000.0).round() / 1000.0,
            )
        }

        for source in SourceColorPrimaries::ALL {
            let reference = source.to_rgb_primaries();
            assert_eq!(
                SourceColorPrimaries::from_chromaticities(reference),
                Some(source)
            );
            let rounded = RgbPrimaries {
                red: round3(reference.red),
                green: round3(reference.green),
                blue: round3(reference.blue),
                white: round3(reference.white),
            };
            assert_eq!(
                SourceColorPrimaries::from_chromaticities(rounded),
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
