use super::{CubicCurve, CubicSegment, RationalCurve, RationalSegment};
use crate::common_traits::{VectorSpace, WithDerivative, WithTwoDerivatives};
use crate::curve::{
    derivatives::{CurveWithDerivative, CurveWithTwoDerivatives},
    Curve, Interval,
};
use core::ops::Deref;

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
