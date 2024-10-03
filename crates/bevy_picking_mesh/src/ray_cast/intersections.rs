use bevy_math::{Mat4, Ray3d, Vec3, Vec3A};
use bevy_reflect::Reflect;
use bevy_render::primitives::Aabb;

/// A ray intersection with a mesh.
#[derive(Debug, Clone, Reflect)]
pub struct RayMeshHit {
    position: Vec3,
    normal: Vec3,
    barycentric_coord: Vec3,
    distance: f32,
    triangle: Option<[Vec3A; 3]>,
    triangle_index: Option<usize>,
}

impl From<PrimitiveIntersection> for RayMeshHit {
    fn from(data: PrimitiveIntersection) -> Self {
        Self {
            position: data.position(),
            normal: data.normal(),
            distance: data.distance(),
            barycentric_coord: Vec3::ZERO,
            triangle: None,
            triangle_index: None,
        }
    }
}

impl RayMeshHit {
    /// Creates a new [`RayMeshHit`] with the given data.
    pub fn new(
        position: Vec3,
        normal: Vec3,
        barycentric: Vec3,
        distance: f32,
        triangle: Option<[Vec3A; 3]>,
        triangle_index: Option<usize>,
    ) -> Self {
        Self {
            position,
            normal,
            barycentric_coord: barycentric,
            distance,
            triangle,
            triangle_index,
        }
    }

    /// Get the intersection data's position.
    #[must_use]
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Get the intersection data's normal.
    #[must_use]
    pub fn normal(&self) -> Vec3 {
        self.normal
    }

    /// Get the intersection data's barycentric coord.
    #[must_use]
    pub fn barycentric_coord(&self) -> Vec3 {
        self.barycentric_coord
    }

    /// Get the intersection data's distance.
    #[must_use]
    pub fn distance(&self) -> f32 {
        self.distance
    }

    /// Get the intersection data's triangle.
    #[must_use]
    pub fn triangle(&self) -> Option<[Vec3A; 3]> {
        self.triangle
    }

    /// Get the intersection data's triangle index.
    #[must_use]
    pub fn triangle_index(&self) -> Option<usize> {
        self.triangle_index
    }
}

/// A ray intersection with a primitive.
pub struct PrimitiveIntersection {
    position: Vec3,
    normal: Vec3,
    distance: f32,
}

impl PrimitiveIntersection {
    /// Creates a new [`PrimitiveIntersection`] with the given data.
    pub fn new(position: Vec3, normal: Vec3, distance: f32) -> Self {
        Self {
            position,
            normal,
            distance,
        }
    }

    /// Get the intersection's position
    #[must_use]
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Get the normal vector of the primitive at the point of intersection
    #[must_use]
    pub fn normal(&self) -> Vec3 {
        self.normal
    }

    /// Get the distance between the ray origin and the intersection position
    #[must_use]
    pub fn distance(&self) -> f32 {
        self.distance
    }
}

// TODO: It'd be nice to use `RayCast3d` from `bevy_math` instead, but it only works on normalized rays.
/// Checks if the ray intersects with an AABB of a mesh, returning `[near, far]` if it does.44
pub(crate) fn ray_aabb_intersection_3d(
    ray: Ray3d,
    aabb: &Aabb,
    model_to_world: &Mat4,
) -> Option<[f32; 2]> {
    // Transform the ray to model space
    let world_to_model = model_to_world.inverse();
    let ray_dir: Vec3A = world_to_model.transform_vector3(*ray.direction).into();
    let ray_dir_recip: Vec3A = ray_dir.recip();
    let ray_origin: Vec3A = world_to_model.transform_point3(ray.origin).into();

    // Check if the ray intersects the mesh's AABB. It's useful to work in model space
    // because we can do an AABB intersection test, instead of an OBB intersection test.

    let t_0: Vec3A = (aabb.min() - ray_origin) * ray_dir_recip;
    let t_1: Vec3A = (aabb.max() - ray_origin) * ray_dir_recip;
    let t_min: Vec3A = t_0.min(t_1);
    let t_max: Vec3A = t_0.max(t_1);

    let mut hit_near = t_min.x;
    let mut hit_far = t_max.x;

    if hit_near > t_max.y || t_min.y > hit_far {
        return None;
    }

    if t_min.y > hit_near {
        hit_near = t_min.y;
    }
    if t_max.y < hit_far {
        hit_far = t_max.y;
    }

    if (hit_near > t_max.z) || (t_min.z > hit_far) {
        return None;
    }

    if t_min.z > hit_near {
        hit_near = t_min.z;
    }
    if t_max.z < hit_far {
        hit_far = t_max.z;
    }
    Some([hit_near, hit_far])
}
