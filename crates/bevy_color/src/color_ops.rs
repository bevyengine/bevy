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

/// Trait with methods for asserting a colorspace is within bounds.
/// 
/// During ordinary usage (e.g. reading images from disk, rendering images, picking colors for UI), colors should always be within their ordinary bounds (such as 0 to 1 for RGB colors).
/// However, some applications, such as high dynamic range rendering or bloom rely on unbounded colors to naturally represent a wider array of choices.
pub trait ClampColor: Sized {
    /// Return a new version of this color clamped, with all fields in bounds.
    fn clamped(&self) -> Self;

    /// Changes all the fields of this color to ensure they are within bounds.
    fn clamp(&mut self);

    /// Are all the fields of this color in bounds?
    fn is_within_bounds(&self) -> bool;
}
