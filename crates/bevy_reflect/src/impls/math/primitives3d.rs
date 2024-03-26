use crate as bevy_reflect;
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_math::{primitives::*, Dir3, Vec3};
use bevy_reflect_derive::impl_reflect;

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
        normal: Dir3,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Line3d {
        direction: Dir3,
    }
);

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math::primitives"]
    struct Segment3d {
        direction: Dir3,
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
    struct Triangle3d {
        vertices: [Vec3; 3],
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
