/// Provides a trait to implement the linear interpolation function.
pub trait Lerp {
    /// The Scalar type used in the linear interpolation function.
    type Scalar;

    /// Performs a linear interpolation between `self` and `rhs` based on the value `s`.
    ///
    /// When `s` is `0.0`, the result will be equal to `self`.  When `s` is `1.0`, the result
    /// will be equal to `rhs`. When `s` is outside of range `[0, 1]`, the result is linearly
    /// extrapolated.
    fn lerp(self, rhs: Self, s: Self::Scalar) -> Self;
}

macro_rules! impl_lerp_for_floats {
    ($type:ident) => {
        impl Lerp for $type {
            type Scalar = $type;

            #[inline(always)]
            fn lerp(self, rhs: Self, s: Self::Scalar) -> Self {
                self + ((rhs - self) * s)
            }
        }
    };
}

impl_lerp_for_floats!(f32);
impl_lerp_for_floats!(f64);

macro_rules! impl_lerp_for_integers {
    ($type: ident, $scalar: ident) => {
        impl Lerp for $type {
            type Scalar = $scalar;

            #[inline(always)]
            fn lerp(self, rhs: Self, s: Self::Scalar) -> Self {
                self + ((rhs - self) as $scalar * s).round() as $type
            }
        }
    };
}

macro_rules! impl_lerp_for_unsigned_integers {
    ($type: ident, $scalar: ident) => {
        impl Lerp for $type {
            type Scalar = $scalar;

            #[inline(always)]
            fn lerp(self, rhs: Self, s: Self::Scalar) -> Self {
                if self <= rhs {
                    self + ((rhs - self) as $scalar * s).round() as $type
                } else {
                    self - ((self - rhs) as $scalar * s).round() as $type
                }
            }
        }
    };
}

impl_lerp_for_integers!(i8, f32);
impl_lerp_for_integers!(i16, f32);
impl_lerp_for_integers!(i32, f32);
impl_lerp_for_integers!(i64, f32);
impl_lerp_for_integers!(i128, f32);

impl_lerp_for_unsigned_integers!(u8, f32);
impl_lerp_for_unsigned_integers!(u16, f32);
impl_lerp_for_unsigned_integers!(u32, f32);
impl_lerp_for_unsigned_integers!(u64, f32);
impl_lerp_for_unsigned_integers!(u128, f32);

#[cfg(test)]
mod tests {
    use crate::lerp::Lerp;

    macro_rules! test_float {
        ($f_name: ident, $t: ty) => {
            #[test]
            fn $f_name() {
                assert_eq!((12 as $t).lerp(24., 0.5), 18.);
                assert_eq!((10 as $t).lerp(20., 0.75), 17.5);
                assert_eq!((30 as $t).lerp(10., 0.5), 20.);

                assert_eq!((50 as $t).lerp(0., 0.), 50.);
                assert_eq!((100 as $t).lerp(0., 1.), 0.);
            }
        };
    }

    macro_rules! test_integer {
        ($f_name: ident, $t: ty) => {
            #[test]
            fn $f_name() {
                assert_eq!((12 as $t).lerp(24, 0.5), 18);
                assert_eq!((10 as $t).lerp(20, 0.75), 18);
                assert_eq!((30 as $t).lerp(10, 0.5), 20);

                assert_eq!((50 as $t).lerp(0, 0.), 50);
                assert_eq!((100 as $t).lerp(0, 1.), 0);
            }
        };
    }

    test_float!(lerp_f32, f32);
    test_float!(lerp_f64, f64);

    test_integer!(lerp_i8, i8);
    test_integer!(lerp_i16, i16);
    test_integer!(lerp_i32, i32);
    test_integer!(lerp_i64, i64);
    test_integer!(lerp_i128, i128);

    test_integer!(lerp_u8, u8);
    test_integer!(lerp_u16, u16);
    test_integer!(lerp_u32, u32);
    test_integer!(lerp_u64, u64);
    test_integer!(lerp_u128, u128);

    #[test]
    fn compare_lerp_types() {
        assert_eq!(12.5_f32.lerp(18.5, 0.3), 12.5_f64.lerp(18.5, 0.3) as f32);

        assert_eq!(
            12_i32.lerp(18, 0.3),
            12_f32.lerp(18_f32, 0.3).round() as i32
        );

        assert_eq!(12_i64.lerp(18, 0.3), 12_i8.lerp(18, 0.3) as i64);

        assert_eq!(18_u64.lerp(12, 0.3), 18_f64.lerp(12., 0.3) as u64);
    }
}
