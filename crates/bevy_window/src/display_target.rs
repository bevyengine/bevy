use bevy_ecs::prelude::Component;

#[cfg(feature = "bevy_reflect")]
use {
    bevy_ecs::prelude::ReflectComponent,
    bevy_reflect::{std_traits::ReflectDefault, Reflect},
};

#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// The display output a [`Window`](crate::Window) requests: a transfer
/// function, a gamut, and the luminance the renderer encodes for.
///
/// This is a request. The surface the window presents to may not support the
/// transfer function or gamut, and the output it gets can differ from what is
/// requested. The wgpu [color space and HDR primer] explains what each
/// backend can present.
///
/// A required component of [`Window`](crate::Window). The default is
/// [`DisplayTarget::SDR_SRGB`]. Bevy never changes the values you set, even
/// when the window moves to another monitor. [`OnMonitor`](crate::OnMonitor)
/// changes on such a move, so watch it to update this component yourself.
///
/// [color space and HDR primer]: https://docs.rs/wgpu/30/wgpu/index.html#surface-color-spaces-and-hdr-output
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
pub struct DisplayTarget {
    /// The luminance of paper white, in nits.
    ///
    /// Paper white is the luminance of a plain white UI element. A tonemapped
    /// value of `1.0` maps to it. [`SDR_SRGB`](Self::SDR_SRGB) uses 100 nits.
    /// [ITU-R BT.2408] recommends 203 nits for HDR television.
    ///
    /// [ITU-R BT.2408]: https://www.itu.int/pub/R-REP-BT.2408
    pub paper_white_nits: f32,
    /// The highest luminance the display can show, in nits.
    ///
    /// On SDR displays this equals [`paper_white_nits`](Self::paper_white_nits).
    /// On HDR displays this is higher, so highlights can exceed paper white.
    pub peak_luminance_nits: f32,
    /// The lowest luminance the display can show, in nits.
    pub min_luminance_nits: f32,
    /// The requested color gamut.
    pub gamut: DisplayGamut,
    /// The requested transfer function.
    pub transfer: DisplayTransfer,
}

impl DisplayTarget {
    /// An sRGB display with standard dynamic range.
    pub const SDR_SRGB: Self = Self {
        paper_white_nits: 100.0,
        peak_luminance_nits: 100.0,
        min_luminance_nits: 0.0,
        gamut: DisplayGamut::Rec709,
        transfer: DisplayTransfer::Srgb,
    };

    /// Returns `self` with [`paper_white_nits`](Self::paper_white_nits) set to
    /// `nits`.
    pub const fn with_paper_white(mut self, nits: f32) -> Self {
        self.paper_white_nits = nits;
        self
    }

    /// Returns `self` with [`peak_luminance_nits`](Self::peak_luminance_nits)
    /// set to `nits`.
    pub const fn with_peak_luminance(mut self, nits: f32) -> Self {
        self.peak_luminance_nits = nits;
        self
    }

    /// Returns `self` with [`min_luminance_nits`](Self::min_luminance_nits)
    /// set to `nits`.
    pub const fn with_min_luminance(mut self, nits: f32) -> Self {
        self.min_luminance_nits = nits;
        self
    }

    /// Returns `self` with [`gamut`](Self::gamut) set to `gamut`.
    pub const fn with_gamut(mut self, gamut: DisplayGamut) -> Self {
        self.gamut = gamut;
        self
    }

    /// Returns `self` with [`transfer`](Self::transfer) set to `transfer`.
    pub const fn with_transfer(mut self, transfer: DisplayTransfer) -> Self {
        self.transfer = transfer;
        self
    }

    /// The 10000 nit maximum of PQ (SMPTE ST 2084), and the largest value
    /// [`sanitized_paper_white_nits`] returns.
    ///
    /// [`sanitized_paper_white_nits`]: Self::sanitized_paper_white_nits
    pub const MAX_PAPER_WHITE_NITS: f32 = 10000.0;

    /// Returns [`paper_white_nits`](Self::paper_white_nits) sanitized for
    /// luminance math.
    ///
    /// Non-finite and non-positive values fall back to the 100 nits of
    /// [`SDR_SRGB`](Self::SDR_SRGB), since scaling by them would give a black
    /// or `NaN` frame. Values above
    /// [`MAX_PAPER_WHITE_NITS`](Self::MAX_PAPER_WHITE_NITS) clamp to it. Other
    /// values return unchanged. This method does not warn.
    pub const fn sanitized_paper_white_nits(&self) -> f32 {
        if !self.paper_white_nits.is_finite() || self.paper_white_nits <= 0.0 {
            Self::SDR_SRGB.paper_white_nits
        } else {
            self.paper_white_nits.min(Self::MAX_PAPER_WHITE_NITS)
        }
    }
}

impl Default for DisplayTarget {
    fn default() -> Self {
        Self::SDR_SRGB
    }
}

/// The color gamut of a display.
///
/// A gamut is the range of colors a display can show. It is set by the
/// display's red, green, and blue primaries, the exact colors its RGB values
/// refer to. Each gamut here uses the D65 white point.
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
pub enum DisplayGamut {
    /// The [ITU-R BT.709](https://registry.color.org/rgb-registry/bt709)
    /// primaries. sRGB uses the same primaries.
    #[default]
    Rec709,
    /// The [Display P3](https://registry.color.org/rgb-registry/displayp3)
    /// primaries, the DCI-P3 primaries with a D65 white point. Wider than
    /// [`Rec709`](Self::Rec709) and narrower than [`Rec2020`](Self::Rec2020).
    DisplayP3,
    /// The [ITU-R BT.2020](https://registry.color.org/rgb-registry/bt2020)
    /// primaries, also known as Rec. 2020. HDR10 uses this gamut.
    Rec2020,
}

/// The transfer function that encodes the output for a display.
///
/// A transfer function maps linear color to the signal the display decodes.
/// Each variant corresponds to a wgpu [`SurfaceColorSpace`], whose docs
/// describe the encoding and the backends that support it.
///
/// [`SurfaceColorSpace`]: https://docs.rs/wgpu/30/wgpu/enum.SurfaceColorSpace.html
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
pub enum DisplayTransfer {
    /// The [sRGB](https://registry.color.org/rgb-registry/srgb) transfer
    /// function of IEC 61966-2-1, with standard dynamic range. wgpu's
    /// [`SurfaceColorSpace::Srgb`].
    ///
    /// [`SurfaceColorSpace::Srgb`]: https://docs.rs/wgpu/30/wgpu/enum.SurfaceColorSpace.html#variant.Srgb
    #[default]
    Srgb,
    /// Linear [scRGB], the linear encoding of IEC 61966-2-2. A value of `1.0`
    /// is 80 nits, and values above `1.0` and below `0.0` are valid. wgpu's
    /// [`SurfaceColorSpace::ExtendedSrgbLinear`].
    ///
    /// scRGB always uses the BT.709 primaries, so [`DisplayTarget::gamut`]
    /// does not apply to this transfer. Colors outside BT.709 are encoded as
    /// out-of-range values.
    ///
    /// [scRGB]: https://en.wikipedia.org/wiki/ScRGB
    /// [`SurfaceColorSpace::ExtendedSrgbLinear`]: https://docs.rs/wgpu/30/wgpu/enum.SurfaceColorSpace.html#variant.ExtendedSrgbLinear
    ScRgbLinear,
    /// The [perceptual quantizer] of SMPTE ST 2084 and [ITU-R BT.2100], used
    /// by HDR10. PQ encodes absolute luminance, with `1.0` at 10000 nits.
    /// HDR10 uses the BT.2020 gamut, so this transfer pairs with
    /// [`DisplayGamut::Rec2020`]. wgpu's [`SurfaceColorSpace::Bt2100Pq`].
    ///
    /// [perceptual quantizer]: https://en.wikipedia.org/wiki/Perceptual_quantizer
    /// [ITU-R BT.2100]: https://www.itu.int/rec/R-REC-BT.2100
    /// [`SurfaceColorSpace::Bt2100Pq`]: https://docs.rs/wgpu/30/wgpu/enum.SurfaceColorSpace.html#variant.Bt2100Pq
    Pq,
    /// Extended-range sRGB, the encoded form of IEC 61966-2-2. The sRGB curve
    /// continues above `1.0` for colors brighter than SDR white and mirrors
    /// below `0.0` for colors outside the gamut.
    ///
    /// Unlike [`ScRgbLinear`](Self::ScRgbLinear), this transfer uses
    /// [`DisplayTarget::gamut`]. With [`DisplayGamut::Rec709`] it is wgpu's
    /// [`SurfaceColorSpace::ExtendedSrgb`], and with
    /// [`DisplayGamut::DisplayP3`] it is [`SurfaceColorSpace::ExtendedDisplayP3`].
    /// wgpu has no extended-range color space for [`DisplayGamut::Rec2020`].
    ///
    /// [`SurfaceColorSpace::ExtendedSrgb`]: https://docs.rs/wgpu/30/wgpu/enum.SurfaceColorSpace.html#variant.ExtendedSrgb
    /// [`SurfaceColorSpace::ExtendedDisplayP3`]: https://docs.rs/wgpu/30/wgpu/enum.SurfaceColorSpace.html#variant.ExtendedDisplayP3
    ExtendedSrgb,
}

impl DisplayTransfer {
    /// Returns `true` if this transfer function has high dynamic range. Every
    /// transfer function except [`Srgb`](Self::Srgb) does.
    pub const fn is_hdr(&self) -> bool {
        matches!(self, Self::ScRgbLinear | Self::Pq | Self::ExtendedSrgb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitized_paper_white_passes_valid_values_through_bit_for_bit() {
        for nits in [0.001, 80.0, 100.0, 203.0, 1000.0, 10000.0] {
            let target = DisplayTarget {
                paper_white_nits: nits,
                ..DisplayTarget::SDR_SRGB
            };
            assert_eq!(
                target.sanitized_paper_white_nits().to_bits(),
                nits.to_bits()
            );
        }
    }

    #[test]
    fn sanitized_paper_white_replaces_degenerate_values() {
        for nits in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -0.0, -50.0] {
            let target = DisplayTarget {
                paper_white_nits: nits,
                ..DisplayTarget::SDR_SRGB
            };
            assert_eq!(target.sanitized_paper_white_nits(), 100.0);
        }
    }

    #[test]
    fn sanitized_paper_white_clamps_to_pq_maximum() {
        let target = DisplayTarget {
            paper_white_nits: 20000.0,
            ..DisplayTarget::SDR_SRGB
        };
        assert_eq!(
            target.sanitized_paper_white_nits(),
            DisplayTarget::MAX_PAPER_WHITE_NITS
        );
    }
}
