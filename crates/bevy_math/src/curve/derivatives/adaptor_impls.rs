//! Implementations of derivatives on curve adaptors. These allow
//! compositionality for derivatives.

use super::{SampleDerivative, SampleTwoDerivatives};
use crate::common_traits::{HasTangent, WithDerivative, WithTwoDerivatives};
use crate::curve::adaptors::{ChainCurve, CurveReparamCurve, LinearReparamCurve};

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
