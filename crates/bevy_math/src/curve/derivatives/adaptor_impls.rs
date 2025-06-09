//! Implementations of derivatives on curve adaptors. These allow
//! compositionality for derivatives.

use super::{SampleDerivative, SampleTwoDerivatives};
use crate::common_traits::{HasTangent, Sum, VectorSpace, WithDerivative, WithTwoDerivatives};
use crate::curve::{
    adaptors::{
        ChainCurve, ConstantCurve, ContinuationCurve, CurveReparamCurve, ForeverCurve, GraphCurve,
        LinearReparamCurve, PingPongCurve, RepeatCurve, ReverseCurve, ZipCurve,
    },
    Curve,
};

// -- ConstantCurve

impl<T> SampleDerivative<T> for ConstantCurve<T>
where
    T: HasTangent + Clone,
{
    fn sample_with_derivative_unchecked(&self, _t: f32) -> WithDerivative<T> {
        WithDerivative {
            value: self.value.clone(),
            derivative: VectorSpace::ZERO,
        }
    }
}

impl<T> SampleTwoDerivatives<T> for ConstantCurve<T>
where
    T: HasTangent + Clone,
{
    fn sample_with_two_derivatives_unchecked(&self, _t: f32) -> WithTwoDerivatives<T> {
        WithTwoDerivatives {
            value: self.value.clone(),
            derivative: VectorSpace::ZERO,
            second_derivative: VectorSpace::ZERO,
        }
    }
}

// -- ChainCurve

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

impl<U, V, S, T, C, D> SampleDerivative<(S, T)> for ZipCurve<S, T, C, D>
where
    U: VectorSpace<Scalar = f32>,
    V: VectorSpace<Scalar = f32>,
    S: HasTangent<Tangent = U>,
    T: HasTangent<Tangent = V>,
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

impl<U, V, S, T, C, D> SampleTwoDerivatives<(S, T)> for ZipCurve<S, T, C, D>
where
    U: VectorSpace<Scalar = f32>,
    V: VectorSpace<Scalar = f32>,
    S: HasTangent<Tangent = U>,
    T: HasTangent<Tangent = V>,
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

impl<V, T, C> SampleDerivative<(f32, T)> for GraphCurve<T, C>
where
    V: VectorSpace<Scalar = f32>,
    T: HasTangent<Tangent = V>,
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

impl<V, T, C> SampleTwoDerivatives<(f32, T)> for GraphCurve<T, C>
where
    V: VectorSpace<Scalar = f32>,
    T: HasTangent<Tangent = V>,
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

        output.derivative = -output.derivative;

        // (Note that the reparametrization that reverses the curve satisfies
        // g'(t)^2 = 1 and g''(t) = 0, so the second derivative is already
        // correct.)

        output
    }
}

// -- CurveReparamCurve

impl<V, T, C, D> SampleDerivative<T> for CurveReparamCurve<T, C, D>
where
    V: VectorSpace<Scalar = f32>,
    T: HasTangent<Tangent = V>,
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

impl<V, T, C, D> SampleTwoDerivatives<T> for CurveReparamCurve<T, C, D>
where
    V: VectorSpace<Scalar = f32>,
    T: HasTangent<Tangent = V>,
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

impl<V, T, C> SampleDerivative<T> for LinearReparamCurve<T, C>
where
    V: VectorSpace<Scalar = f32>,
    T: HasTangent<Tangent = V>,
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

impl<V, T, C> SampleTwoDerivatives<T> for LinearReparamCurve<T, C>
where
    V: VectorSpace<Scalar = f32>,
    T: HasTangent<Tangent = V>,
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

#[cfg(test)]
mod tests {

    use approx::assert_abs_diff_eq;

    use super::*;
    use crate::cubic_splines::{CubicBezier, CubicCardinalSpline, CubicCurve, CubicGenerator};
    use crate::curve::{Curve, CurveExt, Interval};
    use crate::{vec2, Vec2, Vec3};

    fn test_curve() -> CubicCurve<Vec2> {
        let control_pts = [[
            vec2(0.0, 0.0),
            vec2(1.0, 0.0),
            vec2(0.0, 1.0),
            vec2(1.0, 1.0),
        ]];

        CubicBezier::new(control_pts).to_curve().unwrap()
    }

    fn other_test_curve() -> CubicCurve<Vec2> {
        let control_pts = [
            vec2(1.0, 1.0),
            vec2(2.0, 1.0),
            vec2(2.0, 0.0),
            vec2(1.0, 0.0),
        ];

        CubicCardinalSpline::new(0.5, control_pts)
            .to_curve()
            .unwrap()
    }

    fn reparam_curve() -> CubicCurve<f32> {
        let control_pts = [[0.0, 0.25, 0.75, 1.0]];

        CubicBezier::new(control_pts).to_curve().unwrap()
    }

    #[test]
    fn constant_curve() {
        let curve = ConstantCurve::new(Interval::UNIT, Vec3::new(0.2, 1.5, -2.6));
        let jet = curve.sample_with_derivative(0.5).unwrap();
        assert_abs_diff_eq!(jet.derivative, Vec3::ZERO);
    }

    #[test]
    fn chain_curve() {
        let curve1 = test_curve();
        let curve2 = other_test_curve();
        let curve = curve1.by_ref().chain(&curve2).unwrap();

        let jet = curve.sample_with_derivative(0.65).unwrap();
        let true_jet = curve1.sample_with_derivative(0.65).unwrap();
        assert_abs_diff_eq!(jet.value, true_jet.value);
        assert_abs_diff_eq!(jet.derivative, true_jet.derivative);

        let jet = curve.sample_with_derivative(1.1).unwrap();
        let true_jet = curve2.sample_with_derivative(0.1).unwrap();
        assert_abs_diff_eq!(jet.value, true_jet.value);
        assert_abs_diff_eq!(jet.derivative, true_jet.derivative);
    }

    #[test]
    fn continuation_curve() {
        let curve1 = test_curve();
        let curve2 = other_test_curve();
        let curve = curve1.by_ref().chain_continue(&curve2).unwrap();

        let jet = curve.sample_with_derivative(0.99).unwrap();
        let true_jet = curve1.sample_with_derivative(0.99).unwrap();
        assert_abs_diff_eq!(jet.value, true_jet.value);
        assert_abs_diff_eq!(jet.derivative, true_jet.derivative);

        let jet = curve.sample_with_derivative(1.3).unwrap();
        let true_jet = curve2.sample_with_derivative(0.3).unwrap();
        assert_abs_diff_eq!(jet.value, true_jet.value);
        assert_abs_diff_eq!(jet.derivative, true_jet.derivative);
    }

    #[test]
    fn repeat_curve() {
        let curve1 = test_curve();
        let curve = curve1.by_ref().repeat(3).unwrap();

        let jet = curve.sample_with_derivative(0.73).unwrap();
        let true_jet = curve1.sample_with_derivative(0.73).unwrap();
        assert_abs_diff_eq!(jet.value, true_jet.value);
        assert_abs_diff_eq!(jet.derivative, true_jet.derivative);

        let jet = curve.sample_with_derivative(3.5).unwrap();
        let true_jet = curve1.sample_with_derivative(0.5).unwrap();
        assert_abs_diff_eq!(jet.value, true_jet.value);
        assert_abs_diff_eq!(jet.derivative, true_jet.derivative);
    }

    #[test]
    fn forever_curve() {
        let curve1 = test_curve();
        let curve = curve1.by_ref().forever().unwrap();

        let jet = curve.sample_with_derivative(0.12).unwrap();
        let true_jet = curve1.sample_with_derivative(0.12).unwrap();
        assert_abs_diff_eq!(jet.value, true_jet.value);
        assert_abs_diff_eq!(jet.derivative, true_jet.derivative);

        let jet = curve.sample_with_derivative(500.5).unwrap();
        let true_jet = curve1.sample_with_derivative(0.5).unwrap();
        assert_abs_diff_eq!(jet.value, true_jet.value);
        assert_abs_diff_eq!(jet.derivative, true_jet.derivative);
    }

    #[test]
    fn ping_pong_curve() {
        let curve1 = test_curve();
        let curve = curve1.by_ref().ping_pong().unwrap();

        let jet = curve.sample_with_derivative(0.99).unwrap();
        let comparison_jet = curve1.sample_with_derivative(0.99).unwrap();
        assert_abs_diff_eq!(jet.value, comparison_jet.value);
        assert_abs_diff_eq!(jet.derivative, comparison_jet.derivative);

        let jet = curve.sample_with_derivative(1.3).unwrap();
        let comparison_jet = curve1.sample_with_derivative(0.7).unwrap();
        assert_abs_diff_eq!(jet.value, comparison_jet.value);
        assert_abs_diff_eq!(jet.derivative, -comparison_jet.derivative, epsilon = 1.0e-5);
    }

    #[test]
    fn zip_curve() {
        let curve1 = test_curve();
        let curve2 = other_test_curve();
        let curve = curve1.by_ref().zip(&curve2).unwrap();

        let jet = curve.sample_with_derivative(0.7).unwrap();
        let comparison_jet1 = curve1.sample_with_derivative(0.7).unwrap();
        let comparison_jet2 = curve2.sample_with_derivative(0.7).unwrap();
        assert_abs_diff_eq!(jet.value.0, comparison_jet1.value);
        assert_abs_diff_eq!(jet.value.1, comparison_jet2.value);
        let Sum(derivative1, derivative2) = jet.derivative;
        assert_abs_diff_eq!(derivative1, comparison_jet1.derivative);
        assert_abs_diff_eq!(derivative2, comparison_jet2.derivative);
    }

    #[test]
    fn graph_curve() {
        let curve1 = test_curve();
        let curve = curve1.by_ref().graph();

        let jet = curve.sample_with_derivative(0.25).unwrap();
        let comparison_jet = curve1.sample_with_derivative(0.25).unwrap();
        assert_abs_diff_eq!(jet.value.0, 0.25);
        assert_abs_diff_eq!(jet.value.1, comparison_jet.value);
        let Sum(one, derivative) = jet.derivative;
        assert_abs_diff_eq!(one, 1.0);
        assert_abs_diff_eq!(derivative, comparison_jet.derivative);
    }

    #[test]
    fn reverse_curve() {
        let curve1 = test_curve();
        let curve = curve1.by_ref().reverse().unwrap();

        let jet = curve.sample_with_derivative(0.23).unwrap();
        let comparison_jet = curve1.sample_with_derivative(0.77).unwrap();
        assert_abs_diff_eq!(jet.value, comparison_jet.value);
        assert_abs_diff_eq!(jet.derivative, -comparison_jet.derivative);
    }

    #[test]
    fn curve_reparam_curve() {
        let reparam_curve = reparam_curve();
        let reparam_jet = reparam_curve.sample_with_derivative(0.6).unwrap();

        let curve1 = test_curve();
        let curve = curve1.by_ref().reparametrize_by_curve(&reparam_curve);

        let jet = curve.sample_with_derivative(0.6).unwrap();
        let base_jet = curve1
            .sample_with_derivative(reparam_curve.sample(0.6).unwrap())
            .unwrap();
        assert_abs_diff_eq!(jet.value, base_jet.value);
        assert_abs_diff_eq!(jet.derivative, base_jet.derivative * reparam_jet.derivative);
    }

    #[test]
    fn linear_reparam_curve() {
        let curve1 = test_curve();
        let curve = curve1
            .by_ref()
            .reparametrize_linear(Interval::new(0.0, 0.5).unwrap())
            .unwrap();

        let jet = curve.sample_with_derivative(0.23).unwrap();
        let comparison_jet = curve1.sample_with_derivative(0.46).unwrap();
        assert_abs_diff_eq!(jet.value, comparison_jet.value);
        assert_abs_diff_eq!(jet.derivative, comparison_jet.derivative * 2.0);
    }
}
