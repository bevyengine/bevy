use super::{CubicCurve, CubicSegment, RationalCurve, RationalSegment};
use crate::common_traits::{VectorSpace, WithDerivative, WithTwoDerivatives};
use crate::curve::{
    derivatives::{SimpleDerivativeCurve, SimpleTwoDerivativesCurve},
    Curve, Interval,
};

// -- CubicSegment

impl<P: VectorSpace> Curve<P> for CubicSegment<P> {
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> P {
        self.position(t)
    }
}

impl<P: VectorSpace> SimpleDerivativeCurve<P> for CubicSegment<P> {
    #[inline]
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<P> {
        WithDerivative {
            point: self.position(t),
            derivative: self.velocity(t),
        }
    }
}

impl<P: VectorSpace> SimpleTwoDerivativesCurve<P> for CubicSegment<P> {
    #[inline]
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<P> {
        WithTwoDerivatives {
            point: self.position(t),
            derivative: self.velocity(t),
            second_derivative: self.acceleration(t),
        }
    }
}

// -- CubicCurve

impl<P: VectorSpace> Curve<P> for CubicCurve<P> {
    #[inline]
    fn domain(&self) -> Interval {
        // The non-emptiness invariant guarantees that this succeeds.
        Interval::new(0.0, self.segments.len() as f32)
            .expect("CubicCurve is invalid because it has no segments")
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> P {
        self.position(t)
    }
}

impl<P: VectorSpace> SimpleDerivativeCurve<P> for CubicCurve<P> {
    #[inline]
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<P> {
        WithDerivative {
            point: self.position(t),
            derivative: self.velocity(t),
        }
    }
}

impl<P: VectorSpace> SimpleTwoDerivativesCurve<P> for CubicCurve<P> {
    #[inline]
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<P> {
        WithTwoDerivatives {
            point: self.position(t),
            derivative: self.velocity(t),
            second_derivative: self.acceleration(t),
        }
    }
}

// -- RationalSegment

impl<P: VectorSpace> Curve<P> for RationalSegment<P> {
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> P {
        self.position(t)
    }
}

impl<P: VectorSpace> SimpleDerivativeCurve<P> for RationalSegment<P> {
    #[inline]
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<P> {
        WithDerivative {
            point: self.position(t),
            derivative: self.velocity(t),
        }
    }
}

impl<P: VectorSpace> SimpleTwoDerivativesCurve<P> for RationalSegment<P> {
    #[inline]
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<P> {
        WithTwoDerivatives {
            point: self.position(t),
            derivative: self.velocity(t),
            second_derivative: self.acceleration(t),
        }
    }
}

// -- RationalCurve

impl<P: VectorSpace> Curve<P> for RationalCurve<P> {
    #[inline]
    fn domain(&self) -> Interval {
        // The non-emptiness invariant guarantees the success of this.
        Interval::new(0.0, self.length())
            .expect("RationalCurve is invalid because it has zero length")
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> P {
        self.position(t)
    }
}

impl<P: VectorSpace> SimpleDerivativeCurve<P> for RationalCurve<P> {
    #[inline]
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<P> {
        WithDerivative {
            point: self.position(t),
            derivative: self.velocity(t),
        }
    }
}

impl<P: VectorSpace> SimpleTwoDerivativesCurve<P> for RationalCurve<P> {
    #[inline]
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<P> {
        WithTwoDerivatives {
            point: self.position(t),
            derivative: self.velocity(t),
            second_derivative: self.acceleration(t),
        }
    }
}
