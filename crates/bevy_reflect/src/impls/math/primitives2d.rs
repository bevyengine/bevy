use crate as bevy_reflect;
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_math::{primitives::*, Direction2d, Vec2};
use bevy_reflect_derive::impl_reflect;

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Circle {
        radius: f32,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Ellipse {
        pub half_size: Vec2,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Plane2d {
        normal: Direction2d,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Line2d {
        direction: Direction2d,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Segment2d {
        direction: Direction2d,
        half_length: f32,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq)]
    #[type_path = "bevy_math::primitives"]
    struct Polyline2d<const N: usize> {
        vertices: [Vec2; N],
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Triangle2d {
        vertices: [Vec2; 3],
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Rectangle {
        half_size: Vec2,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq)]
    #[type_path = "bevy_math::primitives"]
    struct Polygon<const N: usize> {
        vertices: [Vec2; N],
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct RegularPolygon {
        circumcircle: Circle,
        sides: usize,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Capsule2d {
        radius: f32,
        half_length: f32,
    }
);
