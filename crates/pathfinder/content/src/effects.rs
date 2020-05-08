// pathfinder/content/src/effects.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Special effects that can be applied to layers.

use pathfinder_color::ColorF;
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::vector::Vector2F;
use pathfinder_simd::default::F32x2;

/// This intentionally does not precisely match what Core Graphics does (a
/// Lanczos function), because we don't want any ringing artefacts.
pub static DEFRINGING_KERNEL_CORE_GRAPHICS: DefringingKernel =
    DefringingKernel([0.033165660, 0.102074051, 0.221434336, 0.286651906]);
pub static DEFRINGING_KERNEL_FREETYPE: DefringingKernel =
    DefringingKernel([0.0, 0.031372549, 0.301960784, 0.337254902]);

/// Should match macOS 10.13 High Sierra.
pub static STEM_DARKENING_FACTORS: [f32; 2] = [0.0121, 0.0121 * 1.25];

/// Should match macOS 10.13 High Sierra.
pub const MAX_STEM_DARKENING_AMOUNT: [f32; 2] = [0.3, 0.3];

/// This value is a subjective cutoff. Above this ppem value, no stem darkening is performed.
pub const MAX_STEM_DARKENING_PIXELS_PER_EM: f32 = 72.0;

/// The shader that should be used when compositing this layer onto its destination.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Filter {
    /// No special filter.
    None,

    /// Converts a linear gradient to a radial one.
    RadialGradient {
        /// The line that the circles lie along.
        line: LineSegment2F,
        /// The radii of the circles at the two endpoints.
        radii: F32x2,
        /// The origin of the linearized gradient in the texture.
        uv_origin: Vector2F,
    },

    PatternFilter(PatternFilter),
}

/// Shaders applicable to patterns.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PatternFilter {
    /// Performs postprocessing operations useful for monochrome text.
    Text {
        /// The foreground color of the text.
        fg_color: ColorF,
        /// The background color of the text.
        bg_color: ColorF,
        /// The kernel used for defringing, if subpixel AA is enabled.
        defringing_kernel: Option<DefringingKernel>,
        /// Whether gamma correction is used when compositing.
        ///
        /// If this is enabled, stem darkening is advised.
        gamma_correction: bool,
    },

    /// A blur operation in one direction, either horizontal or vertical.
    ///
    /// To produce a full Gaussian blur, perform two successive blur operations, one in each
    /// direction.
    Blur {
        direction: BlurDirection,
        sigma: f32,
    },
}

/// Blend modes that can be applied to individual paths.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BlendMode {
    // Porter-Duff, supported by GPU blender
    Clear,
    Copy,
    SrcIn,
    SrcOut,
    SrcOver,
    SrcAtop,
    DestIn,
    DestOut,
    DestOver,
    DestAtop,
    Xor,
    Lighter,

    // Others, unsupported by GPU blender
    Darken,
    Lighten,
    Multiply,
    Screen,
    HardLight,
    Overlay,
    ColorDodge,
    ColorBurn,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DefringingKernel(pub [f32; 4]);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BlurDirection {
    X,
    Y,
}

impl Default for BlendMode {
    #[inline]
    fn default() -> BlendMode {
        BlendMode::SrcOver
    }
}

impl Default for Filter {
    #[inline]
    fn default() -> Filter {
        Filter::None
    }
}

impl BlendMode {
    /// Whether the backdrop is irrelevant when applying this blend mode (i.e. destination blend
    /// factor is zero when source alpha is one).
    #[inline]
    pub fn occludes_backdrop(self) -> bool {
        match self {
            BlendMode::SrcOver | BlendMode::Clear => true,
            BlendMode::DestOver |
            BlendMode::DestOut |
            BlendMode::SrcAtop |
            BlendMode::Xor |
            BlendMode::Lighter |
            BlendMode::Lighten |
            BlendMode::Darken |
            BlendMode::Copy |
            BlendMode::SrcIn |
            BlendMode::DestIn |
            BlendMode::SrcOut |
            BlendMode::DestAtop |
            BlendMode::Multiply |
            BlendMode::Screen |
            BlendMode::HardLight |
            BlendMode::Overlay |
            BlendMode::ColorDodge |
            BlendMode::ColorBurn |
            BlendMode::SoftLight |
            BlendMode::Difference |
            BlendMode::Exclusion |
            BlendMode::Hue |
            BlendMode::Saturation |
            BlendMode::Color |
            BlendMode::Luminosity => false,
        }
    }

    /// True if this blend mode does not preserve destination areas outside the source.
    pub fn is_destructive(self) -> bool {
        match self {
            BlendMode::Clear |
            BlendMode::Copy |
            BlendMode::SrcIn |
            BlendMode::DestIn |
            BlendMode::SrcOut |
            BlendMode::DestAtop => true,
            BlendMode::SrcOver |
            BlendMode::DestOver |
            BlendMode::DestOut |
            BlendMode::SrcAtop |
            BlendMode::Xor |
            BlendMode::Lighter |
            BlendMode::Lighten |
            BlendMode::Darken |
            BlendMode::Multiply |
            BlendMode::Screen |
            BlendMode::HardLight |
            BlendMode::Overlay |
            BlendMode::ColorDodge |
            BlendMode::ColorBurn |
            BlendMode::SoftLight |
            BlendMode::Difference |
            BlendMode::Exclusion |
            BlendMode::Hue |
            BlendMode::Saturation |
            BlendMode::Color |
            BlendMode::Luminosity => false,
        }
    }
}
