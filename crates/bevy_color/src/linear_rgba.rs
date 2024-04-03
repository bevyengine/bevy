use crate::{
    color_difference::EuclideanDistance, impl_componentwise_vector_space, Alpha, ClampColor,
    Luminance, Mix, StandardColor,
};
use bevy_math::Vec4;
use bevy_reflect::prelude::*;
use bytemuck::{Pod, Zeroable};

/// Linear RGB color with alpha.
#[doc = include_str!("../docs/conversion.md")]
/// <div>
#[doc = include_str!("../docs/diagrams/model_graph.svg")]
/// </div>
#[derive(Debug, Clone, Copy, PartialEq, Reflect, Pod, Zeroable)]
#[reflect(PartialEq, Default)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[repr(C)]
pub struct LinearRgba {
    /// The red channel. [0.0, 1.0]
    pub red: f32,
    /// The green channel. [0.0, 1.0]
    pub green: f32,
    /// The blue channel. [0.0, 1.0]
    pub blue: f32,
    /// The alpha channel. [0.0, 1.0]
    pub alpha: f32,
}

impl StandardColor for LinearRgba {}

impl_componentwise_vector_space!(LinearRgba, [red, green, blue, alpha]);

impl LinearRgba {
    /// A fully black color with full alpha.
    pub const BLACK: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };

    /// A fully white color with full alpha.
    pub const WHITE: Self = Self {
        red: 1.0,
        green: 1.0,
        blue: 1.0,
        alpha: 1.0,
    };

    /// A fully transparent color.
    pub const NONE: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 0.0,
    };

    /// A fully red color with full alpha.
    pub const RED: Self = Self {
        red: 1.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };

    /// A fully green color with full alpha.
    pub const GREEN: Self = Self {
        red: 0.0,
        green: 1.0,
        blue: 0.0,
        alpha: 1.0,
    };

    /// A fully blue color with full alpha.
    pub const BLUE: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 1.0,
        alpha: 1.0,
    };

    /// An invalid color.
    ///
    /// This type can be used to represent an invalid color value;
    /// in some rendering applications the color will be ignored,
    /// enabling performant hacks like hiding lines by setting their color to `INVALID`.
    pub const NAN: Self = Self {
        red: f32::NAN,
        green: f32::NAN,
        blue: f32::NAN,
        alpha: f32::NAN,
    };

    /// Construct a new [`LinearRgba`] color from components.
    pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Construct a new [`LinearRgba`] color from (r, g, b) components, with the default alpha (1.0).
    ///
    /// # Arguments
    ///
    /// * `red` - Red channel. [0.0, 1.0]
    /// * `green` - Green channel. [0.0, 1.0]
    /// * `blue` - Blue channel. [0.0, 1.0]
    pub const fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    /// Construct a new [`LinearRgba`] color with the same value for all channels and an alpha of 1.0.
    ///
    /// A value of 0.0 is black, and a value of 1.0 is white.
    pub const fn gray(value: f32) -> Self {
        Self {
            red: value,
            green: value,
            blue: value,
            alpha: 1.0,
        }
    }

    /// Return a copy of this color with the red channel set to the given value.
    pub const fn with_red(self, red: f32) -> Self {
        Self { red, ..self }
    }

    /// Return a copy of this color with the green channel set to the given value.
    pub const fn with_green(self, green: f32) -> Self {
        Self { green, ..self }
    }

    /// Return a copy of this color with the blue channel set to the given value.
    pub const fn with_blue(self, blue: f32) -> Self {
        Self { blue, ..self }
    }

    /// Make the color lighter or darker by some amount
    fn adjust_lightness(&mut self, amount: f32) {
        let luminance = self.luminance();
        let target_luminance = (luminance + amount).clamp(0.0, 1.0);
        if target_luminance < luminance {
            let adjustment = (luminance - target_luminance) / luminance;
            self.mix_assign(Self::new(0.0, 0.0, 0.0, self.alpha), adjustment);
        } else if target_luminance > luminance {
            let adjustment = (target_luminance - luminance) / (1. - luminance);
            self.mix_assign(Self::new(1.0, 1.0, 1.0, self.alpha), adjustment);
        }
    }

    /// Converts the color into a [f32; 4] array in RGBA order.
    ///
    /// This is useful for passing the color to a shader.
    pub fn to_f32_array(&self) -> [f32; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }

    /// Converts this color to a u32.
    ///
    /// Maps the RGBA channels in RGBA order to a little-endian byte array (GPUs are little-endian).
    /// `A` will be the most significant byte and `R` the least significant.
    pub fn as_u32(&self) -> u32 {
        u32::from_le_bytes([
            (self.red * 255.0) as u8,
            (self.green * 255.0) as u8,
            (self.blue * 255.0) as u8,
            (self.alpha * 255.0) as u8,
        ])
    }
}

impl Default for LinearRgba {
    /// Construct a new [`LinearRgba`] color with the default values (white with full alpha).
    fn default() -> Self {
        Self::WHITE
    }
}

impl Luminance for LinearRgba {
    /// Luminance calculated using the [CIE XYZ formula](https://en.wikipedia.org/wiki/Relative_luminance).
    #[inline]
    fn luminance(&self) -> f32 {
        self.red * 0.2126 + self.green * 0.7152 + self.blue * 0.0722
    }

    #[inline]
    fn with_luminance(&self, luminance: f32) -> Self {
        let current_luminance = self.luminance();
        let adjustment = luminance / current_luminance;
        Self {
            red: (self.red * adjustment).clamp(0., 1.),
            green: (self.green * adjustment).clamp(0., 1.),
            blue: (self.blue * adjustment).clamp(0., 1.),
            alpha: self.alpha,
        }
    }

    #[inline]
    fn darker(&self, amount: f32) -> Self {
        let mut result = *self;
        result.adjust_lightness(-amount);
        result
    }

    #[inline]
    fn lighter(&self, amount: f32) -> Self {
        let mut result = *self;
        result.adjust_lightness(amount);
        result
    }
}

impl Mix for LinearRgba {
    #[inline]
    fn mix(&self, other: &Self, factor: f32) -> Self {
        let n_factor = 1.0 - factor;
        Self {
            red: self.red * n_factor + other.red * factor,
            green: self.green * n_factor + other.green * factor,
            blue: self.blue * n_factor + other.blue * factor,
            alpha: self.alpha * n_factor + other.alpha * factor,
        }
    }
}

impl Alpha for LinearRgba {
    #[inline]
    fn with_alpha(&self, alpha: f32) -> Self {
        Self { alpha, ..*self }
    }

    #[inline]
    fn alpha(&self) -> f32 {
        self.alpha
    }

    #[inline]
    fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }
}

impl EuclideanDistance for LinearRgba {
    #[inline]
    fn distance_squared(&self, other: &Self) -> f32 {
        let dr = self.red - other.red;
        let dg = self.green - other.green;
        let db = self.blue - other.blue;
        dr * dr + dg * dg + db * db
    }
}

impl ClampColor for LinearRgba {
    fn clamped(&self) -> Self {
        Self {
            red: self.red.clamp(0., 1.),
            green: self.green.clamp(0., 1.),
            blue: self.blue.clamp(0., 1.),
            alpha: self.alpha.clamp(0., 1.),
        }
    }

    fn is_within_bounds(&self) -> bool {
        (0. ..=1.).contains(&self.red)
            && (0. ..=1.).contains(&self.green)
            && (0. ..=1.).contains(&self.blue)
            && (0. ..=1.).contains(&self.alpha)
    }
}

impl From<LinearRgba> for [f32; 4] {
    fn from(color: LinearRgba) -> Self {
        [color.red, color.green, color.blue, color.alpha]
    }
}

impl From<LinearRgba> for Vec4 {
    fn from(color: LinearRgba) -> Self {
        Vec4::new(color.red, color.green, color.blue, color.alpha)
    }
}

#[cfg(feature = "wgpu-types")]
impl From<LinearRgba> for wgpu_types::Color {
    fn from(color: LinearRgba) -> Self {
        wgpu_types::Color {
            r: color.red as f64,
            g: color.green as f64,
            b: color.blue as f64,
            a: color.alpha as f64,
        }
    }
}

// [`LinearRgba`] is intended to be used with shaders
// So it's the only color type that implements [`ShaderType`] to make it easier to use inside shaders
impl encase::ShaderType for LinearRgba {
    type ExtraMetadata = ();

    const METADATA: encase::private::Metadata<Self::ExtraMetadata> = {
        let size =
            encase::private::SizeValue::from(<f32 as encase::private::ShaderSize>::SHADER_SIZE)
                .mul(4);
        let alignment = encase::private::AlignmentValue::from_next_power_of_two_size(size);

        encase::private::Metadata {
            alignment,
            has_uniform_min_alignment: false,
            min_size: size,
            extra: (),
        }
    };

    const UNIFORM_COMPAT_ASSERT: fn() = || {};
}

impl encase::private::WriteInto for LinearRgba {
    fn write_into<B: encase::private::BufferMut>(&self, writer: &mut encase::private::Writer<B>) {
        for el in &[self.red, self.green, self.blue, self.alpha] {
            encase::private::WriteInto::write_into(el, writer);
        }
    }
}

impl encase::private::ReadFrom for LinearRgba {
    fn read_from<B: encase::private::BufferRef>(
        &mut self,
        reader: &mut encase::private::Reader<B>,
    ) {
        let mut buffer = [0.0f32; 4];
        for el in &mut buffer {
            encase::private::ReadFrom::read_from(el, reader);
        }

        *self = LinearRgba {
            red: buffer[0],
            green: buffer[1],
            blue: buffer[2],
            alpha: buffer[3],
        }
    }
}

impl encase::private::CreateFrom for LinearRgba {
    fn create_from<B>(reader: &mut encase::private::Reader<B>) -> Self
    where
        B: encase::private::BufferRef,
    {
        // These are intentionally not inlined in the constructor to make this
        // resilient to internal Color refactors / implicit type changes.
        let red: f32 = encase::private::CreateFrom::create_from(reader);
        let green: f32 = encase::private::CreateFrom::create_from(reader);
        let blue: f32 = encase::private::CreateFrom::create_from(reader);
        let alpha: f32 = encase::private::CreateFrom::create_from(reader);
        LinearRgba {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl encase::ShaderSize for LinearRgba {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euclidean_distance() {
        // White to black
        let a = LinearRgba::new(0.0, 0.0, 0.0, 1.0);
        let b = LinearRgba::new(1.0, 1.0, 1.0, 1.0);
        assert_eq!(a.distance_squared(&b), 3.0);

        // Alpha shouldn't matter
        let a = LinearRgba::new(0.0, 0.0, 0.0, 1.0);
        let b = LinearRgba::new(1.0, 1.0, 1.0, 0.0);
        assert_eq!(a.distance_squared(&b), 3.0);

        // Red to green
        let a = LinearRgba::new(0.0, 0.0, 0.0, 1.0);
        let b = LinearRgba::new(1.0, 0.0, 0.0, 1.0);
        assert_eq!(a.distance_squared(&b), 1.0);
    }

    #[test]
    fn darker_lighter() {
        // Darker and lighter should be commutative.
        let color = LinearRgba::new(0.4, 0.5, 0.6, 1.0);
        let darker1 = color.darker(0.1);
        let darker2 = darker1.darker(0.1);
        let twice_as_dark = color.darker(0.2);
        assert!(darker2.distance_squared(&twice_as_dark) < 0.0001);

        let lighter1 = color.lighter(0.1);
        let lighter2 = lighter1.lighter(0.1);
        let twice_as_light = color.lighter(0.2);
        assert!(lighter2.distance_squared(&twice_as_light) < 0.0001);
    }

    #[test]
    fn test_clamp() {
        let color_1 = LinearRgba::rgb(2., -1., 0.4);
        let color_2 = LinearRgba::rgb(0.031, 0.749, 1.);
        let mut color_3 = LinearRgba::rgb(-1., 1., 1.);

        assert!(!color_1.is_within_bounds());
        assert_eq!(color_1.clamped(), LinearRgba::rgb(1., 0., 0.4));

        assert!(color_2.is_within_bounds());
        assert_eq!(color_2, color_2.clamped());

        color_3.clamp();
        assert!(color_3.is_within_bounds());
        assert_eq!(color_3, LinearRgba::rgb(0., 1., 1.));
    }
}
