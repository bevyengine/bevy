use crate as bevy_reflect;
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_math::{primitives::*, Vec3};
use bevy_reflect_derive::{impl_reflect, impl_reflect_value};

impl_reflect_value!(::bevy_math::primitives::Direction3d(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Sphere {
        radius: f32,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Plane3d {
        normal: Direction3d,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Line3d {
        direction: Direction3d,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Segment3d {
        direction: Direction3d,
        half_length: f32,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq)]
    #[type_path = "bevy_math::primitives"]
    struct Polyline3d<const N: usize> {
        vertices: [Vec3; N],
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Cuboid {
        half_size: Vec3,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Cylinder {
        radius: f32,
        half_height: f32,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Capsule3d {
        radius: f32,
        half_length: f32,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Cone {
        radius: f32,
        height: f32,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct ConicalFrustum {
        radius_top: f32,
        radius_bottom: f32,
        height: f32,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Torus {
        minor_radius: f32,
        major_radius: f32,
    }
);
