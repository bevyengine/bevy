use bevy_math::*;
use bevy_reflect::Reflect;
//use bevy_transform::components::GlobalTransform;
use std::error::Error;
use std::fmt;

pub trait Primitive3d {
    /*
    /// Returns true if this primitive is on the outside (normal direction) of the supplied
    fn outside_plane(
        &self,
        primitive_transform: GlobalTransform,
        plane: Plane,
        plane_transform: GlobalTransform,
    ) -> bool;*/
}

#[derive(Debug, Clone)]
pub enum PrimitiveError {
    MinGreaterThanMax,
    NonPositiveExtents,
}
impl Error for PrimitiveError {}
impl fmt::Display for PrimitiveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PrimitiveError::MinGreaterThanMax => {
                write!(f, "AxisAlignedBox minimums must be smaller than maximums")
            }
            PrimitiveError::NonPositiveExtents => {
                write!(f, "AxisAlignedBox extents must be greater than zero")
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
pub struct Sphere {
    pub origin: Vec3,
    pub radius: f32,
}
impl Primitive3d for Sphere {}

/// An oriented box, unlike an axis aligned box, can be rotated and is not constrained to match the
/// orientation of the coordinate system it is defined in. Internally, this is represented as an
/// axis aligned box with some rotation ([Quat]) applied.
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
pub struct OrientedBox {
    pub aab: AxisAlignedBox,
    pub orientation: Quat,
}
impl Primitive3d for OrientedBox {}

/// An axis aligned box is a box whose axes lie in the x/y/z directions of the coordinate system
/// the box is defined in.
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
pub struct AxisAlignedBox {
    minimums: Vec3,
    maximums: Vec3,
}
impl Primitive3d for AxisAlignedBox {}
impl AxisAlignedBox {
    pub fn from_min_max(minimums: Vec3, maximums: Vec3) -> Result<AxisAlignedBox, PrimitiveError> {
        if (maximums - minimums).min_element() > 0.0 {
            Ok(AxisAlignedBox { minimums, maximums })
        } else {
            Err(PrimitiveError::MinGreaterThanMax)
        }
    }
    pub fn from_extents_origin(
        extents: Vec3,
        origin: Vec3,
    ) -> Result<AxisAlignedBox, PrimitiveError> {
        if extents.min_element() > 0.0 {
            Ok(AxisAlignedBox {
                minimums: origin,
                maximums: extents + origin,
            })
        } else {
            Err(PrimitiveError::NonPositiveExtents)
        }
    }
}

/// A frustum is a truncated pyramid that is used to represent the "volume" of world space that is
/// visible to the camera.
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect_value(PartialEq)]
pub struct Frustum {
    planes: [Plane; 6],
}
impl Primitive3d for Frustum {}
impl Frustum {
    pub fn from_camera_properties(
        &self,
        camera_position: Mat4,
        projection_matrix: Mat4,
    ) -> Frustum {
        let ndc_to_world: Mat4 = camera_position * projection_matrix.inverse();
        // Near/Far, Top/Bottom, Left/Right
        let nbl_world = ndc_to_world.project_point3(Vec3::new(-1.0, -1.0, -1.0));
        let nbr_world = ndc_to_world.project_point3(Vec3::new(1.0, -1.0, -1.0));
        let ntl_world = ndc_to_world.project_point3(Vec3::new(-1.0, 1.0, -1.0));
        let fbl_world = ndc_to_world.project_point3(Vec3::new(-1.0, -1.0, 1.0));
        let ftr_world = ndc_to_world.project_point3(Vec3::new(1.0, 1.0, 1.0));
        let ftl_world = ndc_to_world.project_point3(Vec3::new(-1.0, 1.0, 1.0));
        let fbr_world = ndc_to_world.project_point3(Vec3::new(1.0, -1.0, 1.0));
        let ntr_world = ndc_to_world.project_point3(Vec3::new(1.0, 1.0, -1.0));

        let near_normal = (nbr_world - nbl_world)
            .cross(ntl_world - nbl_world)
            .normalize();
        let far_normal = (fbr_world - ftr_world)
            .cross(ftl_world - ftr_world)
            .normalize();
        let top_normal = (ftl_world - ftr_world)
            .cross(ntr_world - ftr_world)
            .normalize();
        let bottom_normal = (fbl_world - nbl_world)
            .cross(nbr_world - nbl_world)
            .normalize();
        let right_normal = (ntr_world - ftr_world)
            .cross(fbr_world - ftr_world)
            .normalize();
        let left_normal = (ntl_world - nbl_world)
            .cross(fbl_world - nbl_world)
            .normalize();

        let left = Plane {
            point: nbl_world,
            normal: left_normal,
        };
        let right = Plane {
            point: ftr_world,
            normal: right_normal,
        };
        let bottom = Plane {
            point: nbl_world,
            normal: bottom_normal,
        };
        let top = Plane {
            point: ftr_world,
            normal: top_normal,
        };
        let near = Plane {
            point: nbl_world,
            normal: near_normal,
        };
        let far = Plane {
            point: ftr_world,
            normal: far_normal,
        };
        Frustum {
            planes: [left, right, top, bottom, near, far],
        }
    }
}

/// A plane is defined by a point in space and a normal vector at that point.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Plane {
    point: Vec3,
    normal: Vec3,
}
impl Primitive3d for Plane {}
impl Plane {
    /// Generate a plane from three points that lie on the plane.
    pub fn from_points(points: [Vec3; 3]) -> Plane {
        let point = points[1];
        let arm_1 = points[0] - point;
        let arm_2 = points[2] - point;
        let normal = arm_1.cross(arm_2).normalize();
        Plane { point, normal }
    }
    /// Generate a plane from a point on that plane and the normal direction of the plane. The
    /// normal vector does not need to be normalized (length can be != 1).
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Plane {
        Plane {
            point,
            normal: normal.normalize(),
        }
    }
}
