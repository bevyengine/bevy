use super::{CubicSegment, RationalSegment};
use crate::common_traits::{VectorSpace, WithDerivative, WithTwoDerivatives};
use crate::curve::{
    derivatives::{SampleDerivative, SampleTwoDerivatives},
    Curve, Interval,
};

#[cfg(feature = "alloc")]
use super::{CubicCurve, RationalCurve};

// -- CubicSegment

impl<P: VectorSpace<Scalar = f32>> Curve<P> for CubicSegment<P> {
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> P {
        self.position(t)
    }
}

impl<P: VectorSpace<Scalar = f32>> SampleDerivative<P> for CubicSegment<P> {
    #[inline]
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<P> {
        WithDerivative {
            value: self.position(t),
            derivative: self.velocity(t),
        }
    }
}

impl<P: VectorSpace<Scalar = f32>> SampleTwoDerivatives<P> for CubicSegment<P> {
    #[inline]
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<P> {
        WithTwoDerivatives {
            value: self.position(t),
            derivative: self.velocity(t),
            second_derivative: self.acceleration(t),
        }
    }
}

// -- CubicCurve

#[cfg(feature = "alloc")]
impl<P: VectorSpace<Scalar = f32>> Curve<P> for CubicCurve<P> {
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

#[cfg(feature = "alloc")]
impl<P: VectorSpace<Scalar = f32>> SampleDerivative<P> for CubicCurve<P> {
    #[inline]
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<P> {
        WithDerivative {
            value: self.position(t),
            derivative: self.velocity(t),
        }
    }
}

#[cfg(feature = "alloc")]
impl<P: VectorSpace<Scalar = f32>> SampleTwoDerivatives<P> for CubicCurve<P> {
    #[inline]
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<P> {
        WithTwoDerivatives {
            value: self.position(t),
            derivative: self.velocity(t),
            second_derivative: self.acceleration(t),
        }
    }
}

// -- RationalSegment

impl<P: VectorSpace<Scalar = f32>> Curve<P> for RationalSegment<P> {
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> P {
        self.position(t)
    }
}

impl<P: VectorSpace<Scalar = f32>> SampleDerivative<P> for RationalSegment<P> {
    #[inline]
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<P> {
        WithDerivative {
            value: self.position(t),
            derivative: self.velocity(t),
        }
    }
}

impl<P: VectorSpace<Scalar = f32>> SampleTwoDerivatives<P> for RationalSegment<P> {
    #[inline]
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<P> {
        WithTwoDerivatives {
            value: self.position(t),
            derivative: self.velocity(t),
            second_derivative: self.acceleration(t),
        }
    }
}

// -- RationalCurve

#[cfg(feature = "alloc")]
impl<P: VectorSpace<Scalar = f32>> Curve<P> for RationalCurve<P> {
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

#[cfg(feature = "alloc")]
impl<P: VectorSpace<Scalar = f32>> SampleDerivative<P> for RationalCurve<P> {
    #[inline]
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<P> {
        WithDerivative {
            value: self.position(t),
            derivative: self.velocity(t),
        }
    }
}

#[cfg(feature = "alloc")]
impl<P: VectorSpace<Scalar = f32>> SampleTwoDerivatives<P> for RationalCurve<P> {
    #[inline]
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<P> {
        WithTwoDerivatives {
            value: self.position(t),
            derivative: self.velocity(t),
            second_derivative: self.acceleration(t),
        }
    }
}
