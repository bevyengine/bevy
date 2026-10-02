use bevy_color::RgbPrimaries;
use bevy_ecs::prelude::Component;
use bevy_platform::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "bevy_reflect")]
use {
    bevy_ecs::prelude::ReflectComponent,
    bevy_reflect::{std_traits::ReflectDefault, Reflect},
};

#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Requests the color space and luminance for a [`Window`](crate::Window)'s
/// output.
///
/// The default is SDR sRGB. Set [`hdr`](Self::hdr) to get HDR output where
/// the display supports it, or
/// [`color_space_override`](Self::color_space_override) to choose the color
/// space yourself. The surface may not support the request, so the output
/// can differ from it. The wgpu [color space and HDR primer] explains what
/// each backend can present.
///
/// Leave a luminance field `None` unless your app calibrates it. The renderer
/// then uses what the display reports, or a default for the color space.
///
/// A required component of [`Window`](crate::Window). Bevy never writes this
/// component.
///
/// Adding the `Hdr` component to a camera does not give HDR display output.
/// It only changes the format of the texture the camera renders to.
///
/// # Example
///
/// Request HDR output. The color space it gets depends on the platform and
/// the display:
///
/// ```
/// # use bevy_ecs::world::World;
/// # use bevy_window::{DisplayTarget, Window};
/// # let mut world = World::new();
/// world.spawn((
///     Window::default(),
///     DisplayTarget {
///         hdr: true,
///         ..Default::default()
///     },
/// ));
/// ```
///
/// [color space and HDR primer]: https://docs.rs/wgpu/latest/wgpu/index.html#surface-color-spaces-and-hdr-output
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Default, Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct DisplayTarget {
    /// Requests HDR output. Bevy picks the best HDR color space the surface
    /// supports, or SDR when it supports none. Most apps should use this.
    pub hdr: bool,
    /// A color space to use instead of the one Bevy picks, for example to
    /// prefer scRGB over PQ. When `Some`, [`hdr`](Self::hdr) is ignored. When
    /// the surface does not support it, the output is SDR.
    pub color_space_override: Option<SurfaceColorSpace>,
    /// The luminance of a plain white UI element, called paper white, in nits.
    /// A tonemapped value of `1.0` maps to it. On an HDR display, raise it to
    /// make the image brighter.
    ///
    /// SDR uses 100 nits. [ITU-R BT.2408] recommends 203 nits for HDR
    /// television.
    ///
    /// [ITU-R BT.2408]: https://www.itu.int/pub/R-REP-BT.2408
    pub paper_white_nits: Option<f32>,
    /// The highest luminance a highlight can reach, in nits.
    pub peak_luminance_nits: Option<f32>,
    /// The lowest luminance the display can show, in nits.
    pub min_luminance_nits: Option<f32>,
}

/// A color space a surface can present in: a set of primaries, a transfer
/// function, and a range.
///
/// Each variant corresponds to a wgpu [`SurfaceColorSpace`][wgpu], whose docs
/// describe the encoding and the backends that support it. HLG is left out
/// because it needs a scene-referred signal, which Bevy does not produce.
///
/// [wgpu]: https://docs.rs/wgpu/latest/wgpu/enum.SurfaceColorSpace.html
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Default, Debug, PartialEq, Hash, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum SurfaceColorSpace {
    /// [sRGB](https://registry.color.org/rgb-registry/srgb) of IEC 61966-2-1:
    /// the BT.709 primaries with the sRGB transfer function, in standard
    /// dynamic range. wgpu's [`SurfaceColorSpace::Srgb`].
    ///
    /// [`SurfaceColorSpace::Srgb`]: https://docs.rs/wgpu/latest/wgpu/enum.SurfaceColorSpace.html#variant.Srgb
    #[default]
    Srgb,
    /// Linear [scRGB] of IEC 61966-2-2: the BT.709 primaries with a linear
    /// transfer function. A value of `1.0` is 80 nits, and values above `1.0`
    /// and below `0.0` are valid. Colors outside BT.709 are encoded as
    /// out-of-range values. wgpu's [`SurfaceColorSpace::ExtendedSrgbLinear`].
    ///
    /// [scRGB]: https://en.wikipedia.org/wiki/ScRGB
    /// [`SurfaceColorSpace::ExtendedSrgbLinear`]: https://docs.rs/wgpu/latest/wgpu/enum.SurfaceColorSpace.html#variant.ExtendedSrgbLinear
    ScRgbLinear,
    /// HDR10 of [ITU-R BT.2100]: the BT.2020 primaries with the [perceptual
    /// quantizer] of SMPTE ST 2084. PQ encodes absolute luminance, with `1.0`
    /// at 10000 nits. wgpu's [`SurfaceColorSpace::Bt2100Pq`].
    ///
    /// [perceptual quantizer]: https://en.wikipedia.org/wiki/Perceptual_quantizer
    /// [ITU-R BT.2100]: https://www.itu.int/rec/R-REC-BT.2100
    /// [`SurfaceColorSpace::Bt2100Pq`]: https://docs.rs/wgpu/latest/wgpu/enum.SurfaceColorSpace.html#variant.Bt2100Pq
    Pq,
    /// Extended-range sRGB of IEC 61966-2-2: the BT.709 primaries with the
    /// sRGB transfer function continued above `1.0` for colors brighter than
    /// SDR white and mirrored below `0.0` for colors outside the gamut. This
    /// is the web HDR path. wgpu's [`SurfaceColorSpace::ExtendedSrgb`].
    ///
    /// [`SurfaceColorSpace::ExtendedSrgb`]: https://docs.rs/wgpu/latest/wgpu/enum.SurfaceColorSpace.html#variant.ExtendedSrgb
    ExtendedSrgb,
    /// [`ExtendedSrgb`](Self::ExtendedSrgb) with the Display P3 primaries.
    /// wgpu's [`SurfaceColorSpace::ExtendedDisplayP3`].
    ///
    /// [`SurfaceColorSpace::ExtendedDisplayP3`]: https://docs.rs/wgpu/latest/wgpu/enum.SurfaceColorSpace.html#variant.ExtendedDisplayP3
    ExtendedDisplayP3,
}

impl SurfaceColorSpace {
    /// Returns the primaries of this color space.
    pub const fn primaries(&self) -> RgbPrimaries {
        match self {
            Self::Srgb | Self::ScRgbLinear | Self::ExtendedSrgb => RgbPrimaries::BT709,
            Self::Pq => RgbPrimaries::BT2020,
            Self::ExtendedDisplayP3 => RgbPrimaries::DISPLAY_P3,
        }
    }

    /// Returns the transfer function of this color space.
    pub const fn transfer_function(&self) -> TransferFunction {
        match self {
            Self::Srgb => TransferFunction::Srgb,
            Self::ScRgbLinear => TransferFunction::Linear,
            Self::Pq => TransferFunction::Pq,
            Self::ExtendedSrgb | Self::ExtendedDisplayP3 => TransferFunction::ExtendedSrgb,
        }
    }

    /// Returns `true` if this color space has high dynamic range. Every color
    /// space except [`Srgb`](Self::Srgb) does.
    pub const fn is_hdr(&self) -> bool {
        !matches!(self, Self::Srgb)
    }
}

/// The transfer function of a [`SurfaceColorSpace`]: how linear color maps
/// to the signal the display decodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Hash, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum TransferFunction {
    /// The sRGB transfer function of IEC 61966-2-1, over `0.0` to `1.0`.
    Srgb,
    /// No transfer function. The signal is linear light, and values above
    /// `1.0` and below `0.0` are valid.
    Linear,
    /// The perceptual quantizer of SMPTE ST 2084, over absolute luminance up
    /// to 10000 nits.
    Pq,
    /// The sRGB transfer function continued above `1.0` and mirrored below
    /// `0.0`.
    ExtendedSrgb,
}

/// Logs a warning once per call site.
///
/// `bevy_window` has no dependency on `bevy_log`, so this is a local guard
/// over [`log::warn!`].
macro_rules! warn_once {
    ($($arg:tt)+) => {{
        static FIRED: AtomicBool = AtomicBool::new(false);
        if !FIRED.swap(true, Ordering::Relaxed) {
            log::warn!($($arg)+);
        }
    }};
}

/// The paper white of SDR, in nits. sRGB reference viewing conditions specify
/// 80 nits and [ITU-R BT.2035] specifies 100 nits, and 100 is the common
/// choice for SDR content on desktop displays.
///
/// [ITU-R BT.2035]: https://www.itu.int/rec/R-REC-BT.2035
pub const SDR_PAPER_WHITE_NITS: f32 = 100.0;

/// The paper white [ITU-R BT.2408] recommends for PQ, in nits.
///
/// [ITU-R BT.2408]: https://www.itu.int/pub/R-REP-BT.2408
pub const PQ_PAPER_WHITE_NITS: f32 = 203.0;

/// The luminance of signal `1.0` in scRGB and extended sRGB, in nits. The OS
/// maps this signal to its own SDR white, so a paper white of 80 nits means
/// "match the OS SDR white".
pub const SCRGB_REFERENCE_WHITE_NITS: f32 = 80.0;

/// The peak luminance an HDR color space gets when the app has not
/// calibrated it, in nits. Most HDR displays reach at least this.
pub const HDR_PEAK_LUMINANCE_NITS: f32 = 1000.0;

/// The highest luminance a [`DisplayTarget`] field can resolve to, in nits.
/// It is the top of the PQ curve, and no display exceeds it.
pub const MAX_LUMINANCE_NITS: f32 = 10000.0;

impl SurfaceColorSpace {
    /// Returns the paper white this color space uses when the app has not
    /// calibrated it, in nits.
    ///
    /// SDR uses [`SDR_PAPER_WHITE_NITS`]. PQ uses [`PQ_PAPER_WHITE_NITS`]. The
    /// other HDR color spaces use [`SCRGB_REFERENCE_WHITE_NITS`], so
    /// tonemapped white lands on the OS SDR white.
    pub const fn default_paper_white_nits(&self) -> f32 {
        match self {
            Self::Srgb => SDR_PAPER_WHITE_NITS,
            Self::Pq => PQ_PAPER_WHITE_NITS,
            Self::ScRgbLinear | Self::ExtendedSrgb | Self::ExtendedDisplayP3 => {
                SCRGB_REFERENCE_WHITE_NITS
            }
        }
    }

    /// Returns the peak luminance this color space uses when the app has not
    /// calibrated it, in nits. SDR peaks at paper white. HDR uses
    /// [`HDR_PEAK_LUMINANCE_NITS`].
    pub const fn default_peak_luminance_nits(&self) -> f32 {
        match self {
            Self::Srgb => SDR_PAPER_WHITE_NITS,
            _ => HDR_PEAK_LUMINANCE_NITS,
        }
    }

    /// Returns the minimum luminance this color space uses when the app has
    /// not calibrated it, in nits. It is `0.0` for every color space.
    pub const fn default_min_luminance_nits(&self) -> f32 {
        0.0
    }
}

/// A [`DisplayTarget`] after surface negotiation: the color space the output
/// uses and the luminance values the renderer encodes for.
///
/// [`DisplayTarget::resolve`] builds it. Each luminance is the calibrated
/// value when the app set one, else the default for the color space. The
/// default is SDR sRGB at 100 nits.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Default, Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct ResolvedDisplayTarget {
    /// The color space the output uses.
    pub color_space: SurfaceColorSpace,
    /// The luminance of paper white, in nits. See
    /// [`DisplayTarget::paper_white_nits`].
    pub paper_white_nits: f32,
    /// The highest luminance the display can show, in nits. It is at least
    /// [`paper_white_nits`](Self::paper_white_nits).
    pub peak_luminance_nits: f32,
    /// The lowest luminance the display can show, in nits. It is at most
    /// [`paper_white_nits`](Self::paper_white_nits).
    pub min_luminance_nits: f32,
}

impl Default for ResolvedDisplayTarget {
    fn default() -> Self {
        Self {
            color_space: SurfaceColorSpace::Srgb,
            paper_white_nits: SDR_PAPER_WHITE_NITS,
            peak_luminance_nits: SDR_PAPER_WHITE_NITS,
            min_luminance_nits: 0.0,
        }
    }
}

/// Resolves one luminance field of a [`DisplayTarget`].
///
/// `Some` passes through unless it is not finite, fails `is_valid`, or is
/// above [`MAX_LUMINANCE_NITS`]. `None` takes `default`. Each problem warns
/// once, naming the field.
macro_rules! resolve_luminance {
    ($field:ident, $value:expr, $default:expr, $is_valid:expr) => {{
        let default: f32 = $default;
        let is_valid: fn(f32) -> bool = $is_valid;
        match $value {
            None => default,
            Some(value) if !value.is_finite() || !is_valid(value) => {
                warn_once!(
                    "DisplayTarget::{} is {value}, which is not a valid luminance. Using \
                    the default of {default} nits.",
                    stringify!($field)
                );
                default
            }
            Some(value) if value > MAX_LUMINANCE_NITS => {
                warn_once!(
                    "DisplayTarget::{} is {value}, above the {MAX_LUMINANCE_NITS} nits a \
                    display can show. Clamping it.",
                    stringify!($field)
                );
                MAX_LUMINANCE_NITS
            }
            Some(value) => value,
        }
    }};
}

impl DisplayTarget {
    /// Resolves this request for the color space the surface negotiated.
    ///
    /// A calibrated luminance is used as is, unless it is not finite or not
    /// positive (the default is used, with a warning) or above
    /// [`MAX_LUMINANCE_NITS`] (it is clamped, with a warning). A minimum
    /// luminance of `0.0` is valid. An uncalibrated luminance takes the
    /// default of `color_space`: see
    /// [`SurfaceColorSpace::default_paper_white_nits`],
    /// [`SurfaceColorSpace::default_peak_luminance_nits`] and
    /// [`SurfaceColorSpace::default_min_luminance_nits`]. The peak luminance
    /// is raised to at least the paper white, and the minimum luminance is
    /// lowered to at most the paper white.
    pub fn resolve(&self, color_space: SurfaceColorSpace) -> ResolvedDisplayTarget {
        let paper_white_nits = resolve_luminance!(
            paper_white_nits,
            self.paper_white_nits,
            color_space.default_paper_white_nits(),
            |value| value > 0.0
        );
        let peak_luminance_nits = resolve_luminance!(
            peak_luminance_nits,
            self.peak_luminance_nits,
            color_space.default_peak_luminance_nits(),
            |value| value > 0.0
        )
        .max(paper_white_nits);
        let min_luminance_nits = resolve_luminance!(
            min_luminance_nits,
            self.min_luminance_nits,
            color_space.default_min_luminance_nits(),
            |value| value >= 0.0
        )
        .min(paper_white_nits);
        ResolvedDisplayTarget {
            color_space,
            paper_white_nits,
            peak_luminance_nits,
            min_luminance_nits,
        }
    }
}

/// A set of [`SurfaceColorSpace`]s, stored as a bitset.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Default, Debug, PartialEq, Hash, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct SurfaceColorSpaces(u8);

impl SurfaceColorSpaces {
    /// The empty set.
    pub const EMPTY: Self = Self(0);

    /// Every color space, in declaration order.
    const ALL: [SurfaceColorSpace; 5] = [
        SurfaceColorSpace::Srgb,
        SurfaceColorSpace::ScRgbLinear,
        SurfaceColorSpace::Pq,
        SurfaceColorSpace::ExtendedSrgb,
        SurfaceColorSpace::ExtendedDisplayP3,
    ];

    /// The bit for `color_space`. A match rather than a cast, so adding a
    /// variant cannot renumber existing bits.
    const fn bit(color_space: SurfaceColorSpace) -> u8 {
        match color_space {
            SurfaceColorSpace::Srgb => 0b00001,
            SurfaceColorSpace::ScRgbLinear => 0b00010,
            SurfaceColorSpace::Pq => 0b00100,
            SurfaceColorSpace::ExtendedSrgb => 0b01000,
            SurfaceColorSpace::ExtendedDisplayP3 => 0b10000,
        }
    }

    /// Returns this set with `color_space` added.
    pub const fn with(self, color_space: SurfaceColorSpace) -> Self {
        Self(self.0 | Self::bit(color_space))
    }

    /// Returns `true` if `color_space` is a member.
    pub const fn contains(self, color_space: SurfaceColorSpace) -> bool {
        self.0 & Self::bit(color_space) != 0
    }

    /// Iterates the members in [`SurfaceColorSpace`] declaration order.
    pub fn iter(self) -> impl Iterator<Item = SurfaceColorSpace> {
        Self::ALL
            .into_iter()
            .filter(move |&color_space| self.contains(color_space))
    }
}

impl core::fmt::Debug for SurfaceColorSpaces {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// The [`SurfaceColorSpace`] a window's surface uses, and the color spaces
/// it could use.
///
/// [`DisplayTarget`] is a request. The renderer resolves it against what the
/// surface supports and reports the result here.
///
/// The renderer inserts and updates this component. Writing to it has no
/// effect. It is one frame behind the surface and is absent until the
/// surface is configured.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct WindowSurfaceColorSpaces {
    /// The color space the surface uses.
    pub resolved: SurfaceColorSpace,
    /// The color spaces the surface can provide.
    /// [`SurfaceColorSpace::Srgb`] is included when the surface has a format
    /// for the default (automatic) wgpu color space, which is every surface
    /// outside an OS HDR mode that lists formats only in explicit color
    /// spaces.
    pub supported: SurfaceColorSpaces,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_space_set_membership() {
        let set = SurfaceColorSpaces::EMPTY
            .with(SurfaceColorSpace::Srgb)
            .with(SurfaceColorSpace::Pq);
        assert!(set.contains(SurfaceColorSpace::Srgb));
        assert!(set.contains(SurfaceColorSpace::Pq));
        assert!(!set.contains(SurfaceColorSpace::ScRgbLinear));
        assert!(!set.contains(SurfaceColorSpace::ExtendedSrgb));
        assert!(!set.contains(SurfaceColorSpace::ExtendedDisplayP3));
        assert!(!SurfaceColorSpaces::EMPTY.contains(SurfaceColorSpace::Srgb));
        assert_eq!(set.with(SurfaceColorSpace::Pq), set);
    }

    #[test]
    fn color_space_set_iterates_in_declaration_order() {
        let set = SurfaceColorSpaces::EMPTY
            .with(SurfaceColorSpace::ExtendedDisplayP3)
            .with(SurfaceColorSpace::Srgb)
            .with(SurfaceColorSpace::Pq);
        assert!(set.iter().eq([
            SurfaceColorSpace::Srgb,
            SurfaceColorSpace::Pq,
            SurfaceColorSpace::ExtendedDisplayP3,
        ]));
    }

    #[test]
    fn resolve_none_fills_defaults_per_color_space() {
        let expected = [
            (SurfaceColorSpace::Srgb, 100.0, 100.0),
            (SurfaceColorSpace::ScRgbLinear, 80.0, 1000.0),
            (SurfaceColorSpace::Pq, 203.0, 1000.0),
            (SurfaceColorSpace::ExtendedSrgb, 80.0, 1000.0),
            (SurfaceColorSpace::ExtendedDisplayP3, 80.0, 1000.0),
        ];
        for (color_space, paper_white_nits, peak_luminance_nits) in expected {
            assert_eq!(
                DisplayTarget::default().resolve(color_space),
                ResolvedDisplayTarget {
                    color_space,
                    paper_white_nits,
                    peak_luminance_nits,
                    min_luminance_nits: 0.0,
                },
                "{color_space:?}"
            );
        }
        assert_eq!(
            DisplayTarget::default().resolve(SurfaceColorSpace::Srgb),
            ResolvedDisplayTarget::default()
        );
    }

    #[test]
    fn resolve_some_passes_through_bit_for_bit() {
        let target = DisplayTarget {
            paper_white_nits: Some(150.25),
            peak_luminance_nits: Some(1234.5678),
            min_luminance_nits: Some(0.0051),
            ..Default::default()
        };
        let resolved = target.resolve(SurfaceColorSpace::Pq);
        assert_eq!(resolved.paper_white_nits.to_bits(), 150.25f32.to_bits());
        assert_eq!(
            resolved.peak_luminance_nits.to_bits(),
            1234.5678f32.to_bits()
        );
        assert_eq!(resolved.min_luminance_nits.to_bits(), 0.0051f32.to_bits());

        // Zero is a valid minimum luminance.
        let zero_min = DisplayTarget {
            min_luminance_nits: Some(0.0),
            ..Default::default()
        };
        assert_eq!(
            zero_min.resolve(SurfaceColorSpace::Srgb).min_luminance_nits,
            0.0
        );
    }

    #[test]
    fn resolve_degenerate_some_falls_back_to_the_default() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -1.0] {
            let target = DisplayTarget {
                paper_white_nits: Some(bad),
                peak_luminance_nits: Some(bad),
                ..Default::default()
            };
            assert_eq!(
                target.resolve(SurfaceColorSpace::Pq),
                DisplayTarget::default().resolve(SurfaceColorSpace::Pq),
                "{bad}"
            );
        }
        for bad in [f32::NAN, f32::INFINITY, -1.0] {
            let target = DisplayTarget {
                min_luminance_nits: Some(bad),
                ..Default::default()
            };
            assert_eq!(
                target.resolve(SurfaceColorSpace::Srgb).min_luminance_nits,
                0.0,
                "{bad}"
            );
        }
    }

    #[test]
    fn resolve_clamps_to_the_maximum_luminance() {
        let target = DisplayTarget {
            paper_white_nits: Some(20000.0),
            peak_luminance_nits: Some(f32::MAX),
            min_luminance_nits: Some(10001.0),
            ..Default::default()
        };
        assert_eq!(
            target.resolve(SurfaceColorSpace::ScRgbLinear),
            ResolvedDisplayTarget {
                color_space: SurfaceColorSpace::ScRgbLinear,
                paper_white_nits: MAX_LUMINANCE_NITS,
                peak_luminance_nits: MAX_LUMINANCE_NITS,
                min_luminance_nits: MAX_LUMINANCE_NITS,
            }
        );
    }

    #[test]
    fn resolve_raises_peak_to_paper_white() {
        let target = DisplayTarget {
            paper_white_nits: Some(500.0),
            peak_luminance_nits: Some(200.0),
            ..Default::default()
        };
        assert_eq!(
            target.resolve(SurfaceColorSpace::Pq).peak_luminance_nits,
            500.0
        );

        // An uncalibrated peak is raised past its default too.
        let target = DisplayTarget {
            paper_white_nits: Some(300.0),
            ..Default::default()
        };
        assert_eq!(
            target.resolve(SurfaceColorSpace::Srgb).peak_luminance_nits,
            300.0
        );

        // A calibrated peak below the default paper white is raised to it.
        let target = DisplayTarget {
            peak_luminance_nits: Some(150.0),
            ..Default::default()
        };
        assert_eq!(
            target.resolve(SurfaceColorSpace::Pq).peak_luminance_nits,
            203.0
        );
    }

    #[test]
    fn resolve_keeps_a_calibrated_peak_above_the_sdr_paper_white() {
        let target = DisplayTarget {
            peak_luminance_nits: Some(400.0),
            ..Default::default()
        };
        let resolved = target.resolve(SurfaceColorSpace::Srgb);
        assert_eq!(resolved.paper_white_nits, 100.0);
        assert_eq!(resolved.peak_luminance_nits, 400.0);
    }

    #[test]
    fn resolve_lowers_min_to_paper_white() {
        let target = DisplayTarget {
            min_luminance_nits: Some(500.0),
            ..Default::default()
        };
        assert_eq!(
            target.resolve(SurfaceColorSpace::Srgb),
            ResolvedDisplayTarget {
                color_space: SurfaceColorSpace::Srgb,
                paper_white_nits: 100.0,
                peak_luminance_nits: 100.0,
                min_luminance_nits: 100.0,
            }
        );
    }
}
