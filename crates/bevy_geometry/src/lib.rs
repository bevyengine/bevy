use bevy_math::*;
use bevy_reflect::Reflect;
use std::error::Error;
use std::fmt;

pub trait Primitive3d {
    /// Returns true if this primitive is entirely on the outside (in the normal direction) of the
    /// supplied plane.
    fn outside_plane(&self, plane: Plane) -> bool;
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
                write!(
                    f,
                    "AxisAlignedBox minimums must be smaller or equal to the maximums"
                )
            }
            PrimitiveError::NonPositiveExtents => {
                write!(f, "AxisAlignedBox extents must be greater than zero")
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
pub struct Sphere {
    origin: Vec3,
    radius: f32,
}

impl Sphere {
    /// Get the sphere's origin.
    pub fn origin(&self) -> Vec3 {
        self.origin
    }

    /// Get the sphere's radius.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Set the sphere's origin.
    pub fn set_origin(&mut self, origin: Vec3) {
        self.origin = origin;
    }

    /// Set the sphere's radius.
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }
}
impl Primitive3d for Sphere {
    /// Use the sphere's position and radius to determin eif it is entirely on the outside of the
    /// the supplied plane.
    fn outside_plane(&self, plane: Plane) -> bool {
        plane.distance_to_point(self.origin) > self.radius
    }
}

/// An oriented box, unlike an axis aligned box, can be rotated and is not constrained to match the
/// orientation of the coordinate system it is defined in. Internally, this is represented as an
/// axis aligned box with some rotation ([Quat]) applied.
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
pub struct OBB {
    aab: AABB,
    transform: Mat4,
}
impl Primitive3d for OBB {
    fn outside_plane(&self, plane: Plane) -> bool {
        for vertex in self.vertices().iter() {
            if plane.distance_to_point(*vertex) <= 0.0 {
                return false;
            }
        }
        true
    }
}
impl OBB {
    /// An ordered list of the vertices that form the 8 corners of the [AxisAlignedBox].
    /// ```none
    ///     (5)------(1)
    ///      | \      | \
    ///      |  (4)------(0)
    ///      |   |    |   |
    ///     (7)--|---(3)  |
    ///        \ |      \ |
    ///         (6)------(2)
    /// ```
    pub fn vertices(&self) -> [Vec3; 8] {
        let mut vertices = [Vec3::ZERO; 8];
        let aab_vertices = self.aab.vertices();
        for i in 0..vertices.len() {
            vertices[i] = self.transform.project_point3(aab_vertices[i])
        }
        vertices
    }

    /// Set the oriented box's aab.
    pub fn set_aabb(&mut self, aab: AABB) {
        self.aab = aab;
    }

    /// Set the oriented box's transform.
    pub fn set_transform(&mut self, transform: Mat4) {
        self.transform = transform;
    }
    pub fn fast_aabb(&self) -> AABB {
        let vertices = self.vertices();
        let mut max = Vec3::splat(f32::MIN);
        let mut min = Vec3::splat(f32::MAX);
        for vertex in vertices.iter() {
            max = vertex.max(max);
            min = vertex.min(min);
        }
        // Unwrap is okay here because min < max
        AABB::from_min_max(min, max).unwrap()
    }
}

/// An axis aligned box is a box whose axes lie in the x/y/z directions of the coordinate system
/// the box is defined in.
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
pub struct AABB {
    min: Vec3,
    max: Vec3,
}
impl Primitive3d for AABB {
    fn outside_plane(&self, plane: Plane) -> bool {
        for vertex in self.vertices().iter() {
            if plane.distance_to_point(*vertex) <= 0.0 {
                return false;
            }
        }
        true
    }
}
impl AABB {
    /// An ordered list of the vertices that form the 8 corners of the [AxisAlignedBox].
    /// ```none
    ///          (5)------(1)               Y
    ///           | \      | \              |
    ///           |  (4)------(0) MAX       o---X
    ///           |   |    |   |             \
    ///      MIN (7)--|---(3)  |              Z
    ///             \ |      \ |
    ///              (6)------(2)
    /// ```
    pub fn vertices(&self) -> [Vec3; 8] {
        let min = self.min;
        let max = self.max;
        [
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(min.x, min.y, min.z),
        ]
    }
    /// Construct an [AxisAlignedBox] given the coordinates of the minimum and maximum corners.
    pub fn from_min_max(min: Vec3, max: Vec3) -> Result<AABB, PrimitiveError> {
        if (max - min).min_element() >= 0.0 {
            Ok(AABB { min, max })
        } else {
            Err(PrimitiveError::MinGreaterThanMax)
        }
    }
    /// Construct an [AxisAlignedBox] from the origin at the minimum corner, and the extents - the
    /// dimensions of the box in each axis.
    pub fn from_extents_origin(extents: Vec3, origin: Vec3) -> Result<AABB, PrimitiveError> {
        if extents.min_element() > 0.0 {
            Ok(AABB {
                min: origin,
                max: extents + origin,
            })
        } else {
            Err(PrimitiveError::NonPositiveExtents)
        }
    }
    /// Computes the AAB that
    pub fn from_points(points: &[Vec3]) -> AABB {
        let mut max = Vec3::splat(f32::MIN);
        let mut min = Vec3::splat(f32::MAX);
        for &point in points.iter() {
            max = point.max(max);
            min = point.min(min);
        }
        // Unwrap is okay here because min < max
        AABB::from_min_max(min, max).unwrap()
    }
}

/// A frustum is a truncated pyramid that is used to represent the volume of world space that is
/// visible to the camera.
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect_value(PartialEq)]
pub struct Frustum {
    planes: [Plane; 6],
    vertices: [Vec3; 8],
}
impl Primitive3d for Frustum {
    fn outside_plane(&self, plane: Plane) -> bool {
        for vertex in self.vertices().iter() {
            if plane.distance_to_point(*vertex) <= 0.0 {
                return false;
            }
        }
        true
    }
}
impl Frustum {
    fn compute_vertices(camera_position: &Mat4, projection_matrix: &Mat4) -> [Vec3; 8] {
        let ndc_to_world: Mat4 = *camera_position * projection_matrix.inverse();
        [
            ndc_to_world.project_point3(Vec3::new(-1.0, -1.0, -1.0)),
            ndc_to_world.project_point3(Vec3::new(1.0, -1.0, -1.0)),
            ndc_to_world.project_point3(Vec3::new(-1.0, 1.0, -1.0)),
            ndc_to_world.project_point3(Vec3::new(1.0, 1.0, -1.0)),
            ndc_to_world.project_point3(Vec3::new(-1.0, -1.0, 1.0)),
            ndc_to_world.project_point3(Vec3::new(1.0, -1.0, 1.0)),
            ndc_to_world.project_point3(Vec3::new(-1.0, 1.0, 1.0)),
            ndc_to_world.project_point3(Vec3::new(1.0, 1.0, 1.0)),
        ]
    }

    pub fn from_camera_properties(camera_position: &Mat4, projection_matrix: &Mat4) -> Frustum {
        let vertices = Frustum::compute_vertices(camera_position, projection_matrix);
        let [nbl_world, nbr_world, ntl_world, ntr_world, fbl_world, fbr_world, ftl_world, ftr_world] =
            vertices;

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

        let planes = [left, right, top, bottom, near, far];

        Frustum { planes, vertices }
    }

    /// Get a reference to the frustum's vertices. These are given as an ordered list of vertices
    /// that form the 8 corners of a [Frustum].
    /// ```none
    ///     (6)--------------(7)        
    ///      | \    TOP     / |    
    ///      |  (2)------(3)  |
    ///      | L |        | R |  
    ///     (4)  |  NEAR  |  (5)
    ///        \ |        | /
    ///         (0)------(1)
    /// ```
    pub fn vertices(&self) -> &[Vec3; 8] {
        &self.vertices
    }

    /// Get a reference to the frustum's planes.
    pub fn planes(&self) -> &[Plane; 6] {
        &self.planes
    }
}

/// A plane is defined by a point in space and a normal vector at that point.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Plane {
    point: Vec3,
    normal: Vec3,
}
impl Primitive3d for Plane {
    fn outside_plane(&self, plane: Plane) -> bool {
        self.normal == plane.normal && self.distance_to_point(plane.point()) > 0.0
    }
}
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
    /// Returns the nearest distance from the supplied point to this plane. Positive values are in
    /// the direction of the plane's normal (outside), negative values are opposite the direction
    /// of the planes normal (inside).
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + -self.normal.dot(self.point)
    }

    /// Get the plane's point.
    pub fn point(&self) -> Vec3 {
        self.point
    }

    /// Get the plane's normal.
    pub fn normal(&self) -> Vec3 {
        self.normal
    }
}
