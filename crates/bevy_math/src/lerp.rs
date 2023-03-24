
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

// implementation for floats
impl Lerp for f32 {
    type Scalar = f32;

    #[inline(always)]
    fn lerp(self, rhs: Self, s: Self::Scalar) -> Self {
        self + ((rhs - self) * s)
    }
}

impl Lerp for f64 {
    type Scalar = f64;

    #[inline(always)]
    fn lerp(self, rhs: Self, s: Self::Scalar) -> Self {
        self + ((rhs - self) * s)
    }
}

#[cfg(test)]
mod tests {
    use crate::lerp::Lerp;

    #[test]
    fn lerp_f32() {
        assert_eq!(12f32.lerp(24., 0.5), 18.);
        assert_eq!(10f32.lerp(20., 0.75), 17.5);
        assert_eq!(30f32.lerp(10., 0.5), 20.);

        assert_eq!(50f32.lerp(0., 0.), 50.);
        assert_eq!(100f32.lerp(0., 1.), 0.);
    }

    #[test]
    fn lerp_f64() {
        assert_eq!(12f64.lerp(24., 0.5), 18.);
        assert_eq!(10f64.lerp(20., 0.75), 17.5);
        assert_eq!(30f64.lerp(10., 0.5), 20.);

        assert_eq!(50f64.lerp(0., 0.), 50.);
        assert_eq!(100f64.lerp(0., 1.), 0.);
    }
}
