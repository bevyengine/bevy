use bevy_color::RgbPrimaries;
use bevy_ecs::prelude::Component;

#[cfg(feature = "bevy_reflect")]
use {
    bevy_ecs::prelude::ReflectComponent,
    bevy_reflect::{std_traits::ReflectDefault, Reflect},
};

#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// The display output a [`Window`](crate::Window) requests: a dynamic range
/// or color space, and the luminance values the app has calibrated.
///
/// This is a request. The surface the window presents to may not support it,
/// and the output it gets can differ from the request. The wgpu [color space
/// and HDR primer] explains what each backend can present.
///
/// A luminance field is `None` unless the app has calibrated it. Bevy then
/// uses what the display reports, or a default for the color space.
///
/// A required component of [`Window`](crate::Window). The default requests
/// SDR sRGB. Bevy never writes this component.
///
/// This is independent of the `Hdr` component on a camera, which selects the
/// format of the texture the camera renders to.
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
    /// Request HDR output. Bevy picks the best HDR color space the surface
    /// supports. When the surface supports none, the output is SDR.
    pub hdr: bool,
    /// Request one color space instead. When `Some`, [`hdr`](Self::hdr) is
    /// ignored. When the surface does not support it, the output is SDR.
    pub color_space_override: Option<SurfaceColorSpace>,
    /// The luminance of paper white, in nits.
    ///
    /// Paper white is the luminance of a plain white UI element. A tonemapped
    /// value of `1.0` maps to it. SDR uses 100 nits. [ITU-R BT.2408]
    /// recommends 203 nits for HDR television.
    ///
    /// [ITU-R BT.2408]: https://www.itu.int/pub/R-REP-BT.2408
    pub paper_white_nits: Option<f32>,
    /// The highest luminance the display can show, in nits.
    ///
    /// On SDR displays this equals paper white. On HDR displays it is higher,
    /// so highlights can exceed paper white.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_requests_sdr() {
        let target = DisplayTarget::default();
        assert!(!target.hdr);
        assert_eq!(target.color_space_override, None);
    }

    #[test]
    fn only_srgb_is_not_hdr() {
        assert!(!SurfaceColorSpace::Srgb.is_hdr());
        assert!(SurfaceColorSpace::ScRgbLinear.is_hdr());
        assert!(SurfaceColorSpace::Pq.is_hdr());
        assert!(SurfaceColorSpace::ExtendedSrgb.is_hdr());
        assert!(SurfaceColorSpace::ExtendedDisplayP3.is_hdr());
    }

    #[test]
    fn each_color_space_has_primaries_and_a_transfer_function() {
        let expected = [
            (
                SurfaceColorSpace::Srgb,
                RgbPrimaries::BT709,
                TransferFunction::Srgb,
            ),
            (
                SurfaceColorSpace::ScRgbLinear,
                RgbPrimaries::BT709,
                TransferFunction::Linear,
            ),
            (
                SurfaceColorSpace::Pq,
                RgbPrimaries::BT2020,
                TransferFunction::Pq,
            ),
            (
                SurfaceColorSpace::ExtendedSrgb,
                RgbPrimaries::BT709,
                TransferFunction::ExtendedSrgb,
            ),
            (
                SurfaceColorSpace::ExtendedDisplayP3,
                RgbPrimaries::DISPLAY_P3,
                TransferFunction::ExtendedSrgb,
            ),
        ];
        for (color_space, primaries, transfer_function) in expected {
            assert_eq!(color_space.primaries(), primaries);
            assert_eq!(color_space.transfer_function(), transfer_function);
        }
    }
}
