use bevy_math::{Vec3, Vec4};

/// Methods for changing the luminance of a color. Note that these methods are not
/// guaranteed to produce consistent results across color spaces,
/// but will be within a given space.
pub trait Luminance: Sized {
    /// Return the luminance of this color (0.0 - 1.0).
    fn luminance(&self) -> f32;

    /// Return a new version of this color with the given luminance. The resulting color will
    /// be clamped to the valid range for the color space; for some color spaces, clamping
    /// may cause the hue or chroma to change.
    fn with_luminance(&self, value: f32) -> Self;

    /// Return a darker version of this color. The `amount` should be between 0.0 and 1.0.
    /// The amount represents an absolute decrease in luminance, and is distributive:
    /// `color.darker(a).darker(b) == color.darker(a + b)`. Colors are clamped to black
    /// if the amount would cause them to go below black.
    ///
    /// For a relative decrease in luminance, you can simply `mix()` with black.
    fn darker(&self, amount: f32) -> Self;

    /// Return a lighter version of this color. The `amount` should be between 0.0 and 1.0.
    /// The amount represents an absolute increase in luminance, and is distributive:
    /// `color.lighter(a).lighter(b) == color.lighter(a + b)`. Colors are clamped to white
    /// if the amount would cause them to go above white.
    ///
    /// For a relative increase in luminance, you can simply `mix()` with white.
    fn lighter(&self, amount: f32) -> Self;
}

/// Linear interpolation of two colors within a given color space.
pub trait Mix: Sized {
    /// Linearly interpolate between this and another color, by factor.
    /// Factor should be between 0.0 and 1.0.
    fn mix(&self, other: &Self, factor: f32) -> Self;

    /// Linearly interpolate between this and another color, by factor, storing the result
    /// in this color. Factor should be between 0.0 and 1.0.
    fn mix_assign(&mut self, other: Self, factor: f32) {
        *self = self.mix(&other, factor);
    }
}

/// Methods for manipulating alpha values.
pub trait Alpha: Sized {
    /// Return a new version of this color with the given alpha value.
    fn with_alpha(&self, alpha: f32) -> Self;

    /// Return a the alpha component of this color.
    fn alpha(&self) -> f32;

    /// Sets the alpha component of this color.
    fn set_alpha(&mut self, alpha: f32);

    /// Is the alpha component of this color less than or equal to 0.0?
    fn is_fully_transparent(&self) -> bool {
        self.alpha() <= 0.0
    }

    /// Is the alpha component of this color greater than or equal to 1.0?
    fn is_fully_opaque(&self) -> bool {
        self.alpha() >= 1.0
    }
}

/// Trait for manipulating the hue of a color.
pub trait Hue: Sized {
    /// Return a new version of this color with the hue channel set to the given value.
    fn with_hue(&self, hue: f32) -> Self;

    /// Return the hue of this color [0.0, 360.0].
    fn hue(&self) -> f32;

    /// Sets the hue of this color.
    fn set_hue(&mut self, hue: f32);

    /// Return a new version of this color with the hue channel rotated by the given degrees.
    fn rotate_hue(&self, degrees: f32) -> Self {
        let rotated_hue = (self.hue() + degrees).rem_euclid(360.);
        self.with_hue(rotated_hue)
    }
}

/// Trait with methods for asserting a colorspace is within bounds.
///
/// During ordinary usage (e.g. reading images from disk, rendering images, picking colors for UI), colors should always be within their ordinary bounds (such as 0 to 1 for RGB colors).
/// However, some applications, such as high dynamic range rendering or bloom rely on unbounded colors to naturally represent a wider array of choices.
pub trait ClampColor: Sized {
    /// Return a new version of this color clamped, with all fields in bounds.
    fn clamped(&self) -> Self;

    /// Changes all the fields of this color to ensure they are within bounds.
    fn clamp(&mut self) {
        *self = self.clamped();
    }

    /// Are all the fields of this color in bounds?
    fn is_within_bounds(&self) -> bool;
}

/// Trait with methods for converting colors to non-color types
pub trait Convert {
    /// Convert to an f32 array
    fn to_f32_array(self) -> [f32; 4];
    /// Convert to an f32 array without the alpha value
    fn to_alphaless_array(self) -> [f32; 3];
    /// Convert to a Vec4
    fn to_vec4(self) -> Vec4;
    /// Convert to a Vec3
    fn to_vec3(self) -> Vec3;
    /// Convert from an f32 array
    fn from_array(color: [f32; 4]) -> Self;
    /// Convert from an f32 array without the alpha value
    fn from_alphaless_array(color: [f32; 3]) -> Self;
    /// Convert from a Vec4
    fn from_vec4(color: Vec4) -> Self;
    /// Convert from a Vec3
    fn from_vec3(color: Vec3) -> Self;
}

/// Utility function for interpolating hue values. This ensures that the interpolation
/// takes the shortest path around the color wheel, and that the result is always between
/// 0 and 360.
pub(crate) fn lerp_hue(a: f32, b: f32, t: f32) -> f32 {
    let diff = (b - a + 180.0).rem_euclid(360.) - 180.;
    (a + diff * t).rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{testing::assert_approx_eq, Hsla};

    #[test]
    fn test_rotate_hue() {
        let hsla = Hsla::hsl(180.0, 1.0, 0.5);
        assert_eq!(hsla.rotate_hue(90.0), Hsla::hsl(270.0, 1.0, 0.5));
        assert_eq!(hsla.rotate_hue(-90.0), Hsla::hsl(90.0, 1.0, 0.5));
        assert_eq!(hsla.rotate_hue(180.0), Hsla::hsl(0.0, 1.0, 0.5));
        assert_eq!(hsla.rotate_hue(-180.0), Hsla::hsl(0.0, 1.0, 0.5));
        assert_eq!(hsla.rotate_hue(0.0), hsla);
        assert_eq!(hsla.rotate_hue(360.0), hsla);
        assert_eq!(hsla.rotate_hue(-360.0), hsla);
    }

    #[test]
    fn test_hue_wrap() {
        assert_approx_eq!(lerp_hue(10., 20., 0.25), 12.5, 0.001);
        assert_approx_eq!(lerp_hue(10., 20., 0.5), 15., 0.001);
        assert_approx_eq!(lerp_hue(10., 20., 0.75), 17.5, 0.001);

        assert_approx_eq!(lerp_hue(20., 10., 0.25), 17.5, 0.001);
        assert_approx_eq!(lerp_hue(20., 10., 0.5), 15., 0.001);
        assert_approx_eq!(lerp_hue(20., 10., 0.75), 12.5, 0.001);

        assert_approx_eq!(lerp_hue(10., 350., 0.25), 5., 0.001);
        assert_approx_eq!(lerp_hue(10., 350., 0.5), 0., 0.001);
        assert_approx_eq!(lerp_hue(10., 350., 0.75), 355., 0.001);

        assert_approx_eq!(lerp_hue(350., 10., 0.25), 355., 0.001);
        assert_approx_eq!(lerp_hue(350., 10., 0.5), 0., 0.001);
        assert_approx_eq!(lerp_hue(350., 10., 0.75), 5., 0.001);
    }
}
