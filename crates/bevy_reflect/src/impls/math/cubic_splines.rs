use crate as bevy_reflect;
use bevy_math::{cubic_splines::*, VectorSpace};
use bevy_reflect_derive::impl_reflect;

impl_reflect!(
    #[reflect(Debug)]
    #[type_path = "bevy_math::cubic_splines"]
    struct CubicBezier<P: VectorSpace> {
        control_points: Vec<[P; 4]>,
    }
);

impl_reflect!(
    #[reflect(Debug)]
    #[type_path = "bevy_math::cubic_splines"]
    struct CubicHermite<P: VectorSpace> {
        control_points: Vec<(P, P)>,
    }
);

impl_reflect!(
    #[reflect(Debug)]
    #[type_path = "bevy_math::cubic_splines"]
    struct CubicCardinalSpline<P: VectorSpace> {
        tension: f32,
        control_points: Vec<P>,
    }
);

impl_reflect!(
    #[reflect(Debug)]
    #[type_path = "bevy_math::cubic_splines"]
    struct CubicBSpline<P: VectorSpace> {
        control_points: Vec<P>,
    }
);

impl_reflect!(
    #[reflect(Debug)]
    #[type_path = "bevy_math::cubic_splines"]
    struct CubicNurbs<P: VectorSpace> {
        control_points: Vec<P>,
        weights: Vec<f32>,
        knots: Vec<f32>,
    }
);

impl_reflect!(
    #[reflect(Debug)]
    #[type_path = "bevy_math::cubic_splines"]
    struct LinearSpline<P: VectorSpace> {
        points: Vec<P>,
    }
);

impl_reflect!(
    #[reflect(Debug)]
    #[reflect(PartialEq, where P: PartialEq)]
    #[type_path = "bevy_math::cubic_splines"]
    struct CubicSegment<P: VectorSpace> {
        coeff: [P; 4],
    }
);

impl_reflect!(
    #[reflect(Debug)]
    #[reflect(PartialEq, where P: PartialEq)]
    #[type_path = "bevy_math::cubic_splines"]
    struct CubicCurve<P: VectorSpace> {
        segments: Vec<CubicSegment<P>>,
    }
);

impl_reflect!(
    #[reflect(Debug)]
    #[reflect(PartialEq, where P: PartialEq)]
    #[type_path = "bevy_math::cubic_splines"]
    struct RationalSegment<P: VectorSpace> {
        coeff: [P; 4],
        weight_coeff: [f32; 4],
        knot_span: f32,
    }
);

impl_reflect!(
    #[reflect(Debug)]
    #[reflect(PartialEq, where P: PartialEq)]
    #[type_path = "bevy_math::cubic_splines"]
    struct RationalCurve<P: VectorSpace> {
        segments: Vec<RationalSegment<P>>,
    }
);
