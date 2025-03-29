use bevy_math::StableInterpolate;

use crate::{Laba, LinearRgba, Oklaba, Srgba, Xyza};

macro_rules! impl_stable_interpolate_linear {
    ($name: ident $(, $field: ident)*) => {
        impl StableInterpolate for $name {
            fn interpolate_stable(&self, other: &Self, t: f32) -> Self {
                $name {
                    $($field: self.$field.interpolate_stable(&other.$field, t),)*
                }
            }
        }
    };
}

impl_stable_interpolate_linear!(LinearRgba, red, green, blue, alpha);
impl_stable_interpolate_linear!(Srgba, red, green, blue, alpha);
impl_stable_interpolate_linear!(Xyza, x, y, z, alpha);
impl_stable_interpolate_linear!(Laba, lightness, a, b, alpha);
impl_stable_interpolate_linear!(Oklaba, lightness, a, b, alpha);

#[cfg(test)]
mod test {
    use bevy_math::StableInterpolate;

    use crate::Srgba;

    #[test]
    pub fn test_color_stable_interpolate() {
        let b = Srgba::BLACK;
        let w = Srgba::WHITE;
        assert_eq!(
            b.interpolate_stable(&w, 0.5),
            Srgba::new(0.5, 0.5, 0.5, 1.0),
        );
    }
}
