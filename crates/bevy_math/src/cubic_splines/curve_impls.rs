use super::{CubicCurve, CubicSegment, RationalCurve, RationalSegment};
use crate::common_traits::{VectorSpace, WithDerivative, WithTwoDerivatives};
use crate::curve::{
    derivatives::{CurveWithDerivative, CurveWithTwoDerivatives},
    Curve, Interval,
};
use core::ops::Deref;

// NB: This should have the same derives as the other wrapper structs.
/// A wrapper which effectively makes a type `Deref` to itself to avoid code
/// duplication.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
struct Owned<T>(T);

impl<T> Deref for Owned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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

/// Wrapper struct for a [`RationalSegment`] that samples the velocity along
/// with the position.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct RationalSegmentDerivative<P, D>(D)
where
    P: VectorSpace,
    D: Deref<Target = RationalSegment<P>>;

impl<P, D> Curve<WithDerivative<P>> for RationalSegmentDerivative<P, D>
where
    P: VectorSpace,
    D: Deref<Target = RationalSegment<P>>,
{
    fn domain(&self) -> Interval {
        self.0.domain()
    }

    fn sample_unchecked(&self, t: f32) -> WithDerivative<P> {
        WithDerivative {
            point: self.0.position(t),
            derivative: self.0.velocity(t),
        }
    }
}

impl<P: VectorSpace> CurveWithDerivative<P> for RationalSegment<P> {
    fn with_derivative(self) -> impl Curve<WithDerivative<P>> {
        RationalSegmentDerivative(Owned(self))
    }
}

impl<P, D> CurveWithDerivative<P> for D
where
    P: VectorSpace,
    D: Deref<Target = RationalSegment<P>>,
{
    fn with_derivative(self) -> impl Curve<WithDerivative<P>> {
        RationalSegmentDerivative(self)
    }
}

/// Wrapper struct for a [`RationalSegment`] that samples the velocity and
/// acceleration along with the position.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct RationalSegmentTwoDerivatives<P, D>(D)
where
    P: VectorSpace,
    D: Deref<Target = RationalSegment<P>>;

impl<P, D> Curve<WithTwoDerivatives<P>> for RationalSegmentTwoDerivatives<P, D>
where
    P: VectorSpace,
    D: Deref<Target = RationalSegment<P>>,
{
    fn domain(&self) -> Interval {
        self.0.domain()
    }

    fn sample_unchecked(&self, t: f32) -> WithTwoDerivatives<P> {
        WithTwoDerivatives {
            point: self.0.position(t),
            derivative: self.0.velocity(t),
            second_derivative: self.0.acceleration(t),
        }
    }
}

impl<P: VectorSpace> CurveWithTwoDerivatives<P> for RationalSegment<P> {
    fn with_two_derivatives(self) -> impl Curve<WithTwoDerivatives<P>> {
        RationalSegmentTwoDerivatives(Owned(self))
    }
}

impl<P, D> CurveWithTwoDerivatives<P> for D
where
    P: VectorSpace,
    D: Deref<Target = RationalSegment<P>>,
{
    fn with_two_derivatives(self) -> impl Curve<WithTwoDerivatives<P>> {
        RationalSegmentTwoDerivatives(self)
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
