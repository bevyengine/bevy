use bevy_math::{StableInterpolate, VectorSpace};

use crate::{Laba, LinearRgba, Oklaba, Srgba, Xyza};

macro_rules! impl_stable_interpolate_linear {
    ($($name: ident),*) => {
        $(impl StableInterpolate for $name {
            fn interpolate_stable(&self, other: &Self, t: f32) -> Self {
                $name::lerp(*self, *other, t)
            }
        })*
    };
}

impl_stable_interpolate_linear! {
    LinearRgba, Srgba, Xyza, Laba, Oklaba
}

#[cfg(test)]
mod test {
    use bevy_math::StableInterpolate;

    use crate::{Gray, Laba, LinearRgba, Oklaba, Srgba, Xyza};

    #[test]
    pub fn test_color_stable_interpolate() {
        let b = Srgba::BLACK;
        let w = Srgba::WHITE;
        assert_eq!(
            b.interpolate_stable(&w, 0.5),
            Srgba::new(0.5, 0.5, 0.5, 1.0),
        );

        let b = LinearRgba::BLACK;
        let w = LinearRgba::WHITE;
        assert_eq!(
            b.interpolate_stable(&w, 0.5),
            LinearRgba::new(0.5, 0.5, 0.5, 1.0),
        );

        let b = Xyza::BLACK;
        let w = Xyza::WHITE;
        assert_eq!(b.interpolate_stable(&w, 0.5), Xyza::gray(0.5),);

        let b = Laba::BLACK;
        let w = Laba::WHITE;
        assert_eq!(b.interpolate_stable(&w, 0.5), Laba::new(0.5, 0.0, 0.0, 1.0),);

        let b = Oklaba::BLACK;
        let w = Oklaba::WHITE;
        assert_eq!(b.interpolate_stable(&w, 0.5), Oklaba::gray(0.5),);
    }
}
