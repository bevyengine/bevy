//! Implementations of derivatives on curve adaptors. These allow
//! compositionality for derivatives.

use super::{SampleDerivative, SampleTwoDerivatives};
use crate::common_traits::{HasTangent, Sum, VectorSpace, WithDerivative, WithTwoDerivatives};
use crate::curve::{
    adaptors::{
        ChainCurve, ContinuationCurve, CurveReparamCurve, ForeverCurve, GraphCurve,
        LinearReparamCurve, PingPongCurve, RepeatCurve, ReverseCurve, ZipCurve,
    },
    Curve,
};

// -- ChainCurve (chaining derivative curves)

impl<T, C, D> SampleDerivative<T> for ChainCurve<T, C, D>
where
    T: HasTangent,
    C: SampleDerivative<T>,
    D: SampleDerivative<T>,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<T> {
        if t > self.first.domain().end() {
            self.second.sample_with_derivative_unchecked(
                // `t - first.domain.end` computes the offset into the domain of the second.
                t - self.first.domain().end() + self.second.domain().start(),
            )
        } else {
            self.first.sample_with_derivative_unchecked(t)
        }
    }
}

impl<T, C, D> SampleTwoDerivatives<T> for ChainCurve<T, C, D>
where
    T: HasTangent,
    C: SampleTwoDerivatives<T>,
    D: SampleTwoDerivatives<T>,
{
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<T> {
        if t > self.first.domain().end() {
            self.second.sample_with_two_derivatives_unchecked(
                // `t - first.domain.end` computes the offset into the domain of the second.
                t - self.first.domain().end() + self.second.domain().start(),
            )
        } else {
            self.first.sample_with_two_derivatives_unchecked(t)
        }
    }
}

// -- ContinuationCurve

impl<T, C, D> SampleDerivative<T> for ContinuationCurve<T, C, D>
where
    T: VectorSpace,
    C: SampleDerivative<T>,
    D: SampleDerivative<T>,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<T> {
        if t > self.first.domain().end() {
            let mut output = self.second.sample_with_derivative_unchecked(
                // `t - first.domain.end` computes the offset into the domain of the second.
                t - self.first.domain().end() + self.second.domain().start(),
            );
            output.value = output.value + self.offset;
            output
        } else {
            self.first.sample_with_derivative_unchecked(t)
        }
    }
}

impl<T, C, D> SampleTwoDerivatives<T> for ContinuationCurve<T, C, D>
where
    T: VectorSpace,
    C: SampleTwoDerivatives<T>,
    D: SampleTwoDerivatives<T>,
{
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<T> {
        if t > self.first.domain().end() {
            let mut output = self.second.sample_with_two_derivatives_unchecked(
                // `t - first.domain.end` computes the offset into the domain of the second.
                t - self.first.domain().end() + self.second.domain().start(),
            );
            output.value = output.value + self.offset;
            output
        } else {
            self.first.sample_with_two_derivatives_unchecked(t)
        }
    }
}

// -- RepeatCurve

impl<T, C> SampleDerivative<T> for RepeatCurve<T, C>
where
    T: HasTangent,
    C: SampleDerivative<T>,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<T> {
        let t = self.base_curve_sample_time(t);
        self.curve.sample_with_derivative_unchecked(t)
    }
}

impl<T, C> SampleTwoDerivatives<T> for RepeatCurve<T, C>
where
    T: HasTangent,
    C: SampleTwoDerivatives<T>,
{
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<T> {
        let t = self.base_curve_sample_time(t);
        self.curve.sample_with_two_derivatives_unchecked(t)
    }
}

// -- ForeverCurve

impl<T, C> SampleDerivative<T> for ForeverCurve<T, C>
where
    T: HasTangent,
    C: SampleDerivative<T>,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<T> {
        let t = self.base_curve_sample_time(t);
        self.curve.sample_with_derivative_unchecked(t)
    }
}

impl<T, C> SampleTwoDerivatives<T> for ForeverCurve<T, C>
where
    T: HasTangent,
    C: SampleTwoDerivatives<T>,
{
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<T> {
        let t = self.base_curve_sample_time(t);
        self.curve.sample_with_two_derivatives_unchecked(t)
    }
}

// -- PingPongCurve

impl<T, C> SampleDerivative<T> for PingPongCurve<T, C>
where
    T: HasTangent,
    C: SampleDerivative<T>,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<T> {
        if t > self.curve.domain().end() {
            let t = self.curve.domain().end() * 2.0 - t;
            // The derivative of the preceding expression is -1, so the chain
            // rule implies the derivative should be negated.
            let mut output = self.curve.sample_with_derivative_unchecked(t);
            output.derivative = -output.derivative;
            output
        } else {
            self.curve.sample_with_derivative_unchecked(t)
        }
    }
}

impl<T, C> SampleTwoDerivatives<T> for PingPongCurve<T, C>
where
    T: HasTangent,
    C: SampleTwoDerivatives<T>,
{
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<T> {
        if t > self.curve.domain().end() {
            let t = self.curve.domain().end() * 2.0 - t;
            // See the implementation on `ReverseCurve` for an explanation of
            // why this is correct.
            let mut output = self.curve.sample_with_two_derivatives_unchecked(t);
            output.derivative = -output.derivative;
            output
        } else {
            self.curve.sample_with_two_derivatives_unchecked(t)
        }
    }
}

// -- ZipCurve

impl<S, T, C, D> SampleDerivative<(S, T)> for ZipCurve<S, T, C, D>
where
    S: HasTangent,
    T: HasTangent,
    C: SampleDerivative<S>,
    D: SampleDerivative<T>,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<(S, T)> {
        let first_output = self.first.sample_with_derivative_unchecked(t);
        let second_output = self.second.sample_with_derivative_unchecked(t);
        WithDerivative {
            value: (first_output.value, second_output.value),
            derivative: Sum(first_output.derivative, second_output.derivative),
        }
    }
}

impl<S, T, C, D> SampleTwoDerivatives<(S, T)> for ZipCurve<S, T, C, D>
where
    S: HasTangent,
    T: HasTangent,
    C: SampleTwoDerivatives<S>,
    D: SampleTwoDerivatives<T>,
{
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<(S, T)> {
        let first_output = self.first.sample_with_two_derivatives_unchecked(t);
        let second_output = self.second.sample_with_two_derivatives_unchecked(t);
        WithTwoDerivatives {
            value: (first_output.value, second_output.value),
            derivative: Sum(first_output.derivative, second_output.derivative),
            second_derivative: Sum(
                first_output.second_derivative,
                second_output.second_derivative,
            ),
        }
    }
}

// -- GraphCurve

impl<T, C> SampleDerivative<(f32, T)> for GraphCurve<T, C>
where
    T: HasTangent,
    C: SampleDerivative<T>,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<(f32, T)> {
        let output = self.base.sample_with_derivative_unchecked(t);
        WithDerivative {
            value: (t, output.value),
            derivative: Sum(1.0, output.derivative),
        }
    }
}

impl<T, C> SampleTwoDerivatives<(f32, T)> for GraphCurve<T, C>
where
    T: HasTangent,
    C: SampleTwoDerivatives<T>,
{
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<(f32, T)> {
        let output = self.base.sample_with_two_derivatives_unchecked(t);
        WithTwoDerivatives {
            value: (t, output.value),
            derivative: Sum(1.0, output.derivative),
            second_derivative: Sum(0.0, output.second_derivative),
        }
    }
}

// -- ReverseCurve

impl<T, C> SampleDerivative<T> for ReverseCurve<T, C>
where
    T: HasTangent,
    C: SampleDerivative<T>,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<T> {
        // This gets almost the correct value, but we haven't accounted for the
        // reversal of orientation yet.
        let mut output = self
            .curve
            .sample_with_derivative_unchecked(self.domain().end() - (t - self.domain().start()));

        output.derivative = -output.derivative;

        output
    }
}

impl<T, C> SampleTwoDerivatives<T> for ReverseCurve<T, C>
where
    T: HasTangent,
    C: SampleTwoDerivatives<T>,
{
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<T> {
        // This gets almost the correct value, but we haven't accounted for the
        // reversal of orientation yet.
        let mut output = self.curve.sample_with_two_derivatives_unchecked(
            self.domain().end() - (t - self.domain().start()),
        );

        output.derivative = output.derivative * -1.0;

        // (Note that the reparametrization that reverses the curve satisfies
        // g'(t)^2 = 1 and g''(t) = 0, so the second derivative is already
        // correct.)

        output
    }
}

// -- CurveReparamCurve (chain rule)

impl<T, C, D> SampleDerivative<T> for CurveReparamCurve<T, C, D>
where
    T: HasTangent,
    C: SampleDerivative<T>,
    D: SampleDerivative<f32>,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<T> {
        // This curve is r(t) = f(g(t)), where f(t) is `self.base` and g(t)
        // is `self.reparam_curve`.

        // Start by computing g(t) and g'(t).
        let reparam_output = self.reparam_curve.sample_with_derivative_unchecked(t);

        // Compute:
        // - value: f(g(t))
        // - derivative: f'(g(t))
        let mut output = self
            .base
            .sample_with_derivative_unchecked(reparam_output.value);

        // Do the multiplication part of the chain rule.
        output.derivative = output.derivative * reparam_output.derivative;

        // value: f(g(t)), derivative: f'(g(t)) g'(t)
        output
    }
}

impl<T, C, D> SampleTwoDerivatives<T> for CurveReparamCurve<T, C, D>
where
    T: HasTangent,
    C: SampleTwoDerivatives<T>,
    D: SampleTwoDerivatives<f32>,
{
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<T> {
        // This curve is r(t) = f(g(t)), where f(t) is `self.base` and g(t)
        // is `self.reparam_curve`.

        // Start by computing g(t), g'(t), g''(t).
        let reparam_output = self.reparam_curve.sample_with_two_derivatives_unchecked(t);

        // Compute:
        // - value: f(g(t))
        // - derivative: f'(g(t))
        // - second derivative: f''(g(t))
        let mut output = self
            .base
            .sample_with_two_derivatives_unchecked(reparam_output.value);

        // Set the second derivative according to the chain and product rules
        // r''(t) = f''(g(t)) g'(t)^2 + f'(g(t)) g''(t)
        output.second_derivative = (output.second_derivative
            * (reparam_output.derivative * reparam_output.derivative))
            + (output.derivative * reparam_output.second_derivative);

        // Set the first derivative according to the chain rule
        // r'(t) = f'(g(t)) g'(t)
        output.derivative = output.derivative * reparam_output.derivative;

        output
    }
}

// -- LinearReparamCurve

impl<T, C> SampleDerivative<T> for LinearReparamCurve<T, C>
where
    T: HasTangent,
    C: SampleDerivative<T>,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<T> {
        // This curve is r(t) = f(g(t)), where f(t) is `self.base` and g(t) is
        // the linear map bijecting `self.new_domain` onto `self.base.domain()`.

        // The invariants imply this unwrap always succeeds.
        let g = self.new_domain.linear_map_to(self.base.domain()).unwrap();

        // Compute g'(t) from the domain lengths.
        let g_derivative = self.base.domain().length() / self.new_domain.length();

        // Compute:
        // - value: f(g(t))
        // - derivative: f'(g(t))
        let mut output = self.base.sample_with_derivative_unchecked(g(t));

        // Adjust the derivative according to the chain rule.
        output.derivative = output.derivative * g_derivative;

        output
    }
}

impl<T, C> SampleTwoDerivatives<T> for LinearReparamCurve<T, C>
where
    T: HasTangent,
    C: SampleTwoDerivatives<T>,
{
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<T> {
        // This curve is r(t) = f(g(t)), where f(t) is `self.base` and g(t) is
        // the linear map bijecting `self.new_domain` onto `self.base.domain()`.

        // The invariants imply this unwrap always succeeds.
        let g = self.new_domain.linear_map_to(self.base.domain()).unwrap();

        // Compute g'(t) from the domain lengths.
        let g_derivative = self.base.domain().length() / self.new_domain.length();

        // Compute:
        // - value: f(g(t))
        // - derivative: f'(g(t))
        // - second derivative: f''(g(t))
        let mut output = self.base.sample_with_two_derivatives_unchecked(g(t));

        // Set the second derivative according to the chain and product rules
        // r''(t) = f''(g(t)) g'(t)^2  (g''(t) = 0)
        output.second_derivative = output.second_derivative * (g_derivative * g_derivative);

        // Set the first derivative according to the chain rule
        // r'(t) = f'(g(t)) g'(t)
        output.derivative = output.derivative * g_derivative;

        output
    }
}
