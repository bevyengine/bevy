use bevy_math::{Dir3, Mat4, Ray3d, Vec3, Vec3A};
use bevy_reflect::Reflect;
use bevy_render::{
    mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues},
    primitives::Aabb,
};
use bevy_utils::tracing::{error, warn};

use super::Backfaces;

/// A ray intersection with a mesh.
#[derive(Debug, Clone, Reflect)]
pub struct RayMeshHit {
    /// The point of intersection in world space.
    pub point: Vec3,
    /// The normal vector of the triangle at the point of intersection. Not guaranteed to be normalized for scaled meshes.
    pub normal: Vec3,
    /// The barycentric coordinates of the intersection.
    pub barycentric_coords: Vec3,
    /// The distance from the ray origin to the intersection point.
    pub distance: f32,
    /// The vertices of the triangle that was hit.
    pub triangle: Option<[Vec3A; 3]>,
    /// The index of the triangle that was hit.
    pub triangle_index: Option<usize>,
}

/// A hit result from a ray cast on a triangle.
#[derive(Default, Debug)]
pub struct RayTriangleHit {
    pub distance: f32,
    pub barycentric_coords: (f32, f32),
}

/// Casts a ray on a mesh, and returns the intersection.
pub(super) fn ray_intersection_over_mesh(
    mesh: &Mesh,
    mesh_transform: &Mat4,
    ray: Ray3d,
    backface_culling: Backfaces,
) -> Option<RayMeshHit> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        error!(
            "Invalid intersection check: `TriangleList` is the only supported `PrimitiveTopology`"
        );
        return None;
    }

    // Get the vertex positions and normals from the mesh.
    let vertex_positions: &Vec<[f32; 3]> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        None => {
            error!("Mesh does not contain vertex positions");
            return None;
        }
        Some(vertex_values) => match &vertex_values {
            VertexAttributeValues::Float32x3(positions) => positions,
            _ => {
                error!("Unexpected types in {:?}", Mesh::ATTRIBUTE_POSITION);
                return None;
            }
        },
    };
    let vertex_normals: Option<&[[f32; 3]]> =
        if let Some(normal_values) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
            match &normal_values {
                VertexAttributeValues::Float32x3(normals) => Some(normals),
                _ => None,
            }
        } else {
            None
        };

    if let Some(indices) = &mesh.indices() {
        match indices {
            Indices::U16(vertex_indices) => ray_mesh_intersection(
                ray,
                mesh_transform,
                vertex_positions,
                vertex_normals,
                Some(vertex_indices),
                backface_culling,
            ),
            Indices::U32(vertex_indices) => ray_mesh_intersection(
                ray,
                mesh_transform,
                vertex_positions,
                vertex_normals,
                Some(vertex_indices),
                backface_culling,
            ),
        }
    } else {
        ray_mesh_intersection(
            ray,
            mesh_transform,
            vertex_positions,
            vertex_normals,
            None::<&[usize]>,
            backface_culling,
        )
    }
}

/// Checks if a ray intersects a mesh, and returns the nearest intersection if one exists.
pub fn ray_mesh_intersection<Index: Clone + Copy>(
    ray: Ray3d,
    mesh_transform: &Mat4,
    vertex_positions: &[[f32; 3]],
    vertex_normals: Option<&[[f32; 3]]>,
    indices: Option<&[Index]>,
    backface_culling: Backfaces,
) -> Option<RayMeshHit>
where
    usize: TryFrom<Index>,
{
    // The ray cast can hit the same mesh many times, so we need to track which hit is
    // closest to the camera, and record that.
    let mut closest_hit_distance = f32::MAX;
    let mut closest_hit = None;

    let world_to_mesh = mesh_transform.inverse();

    let mesh_space_ray = Ray3d::new(
        world_to_mesh.transform_point3(ray.origin),
        Dir3::new(world_to_mesh.transform_vector3(*ray.direction)).ok()?,
    );

    if let Some(indices) = indices {
        // Make sure this chunk has 3 vertices to avoid a panic.
        if indices.len() % 3 != 0 {
            warn!("Index list not a multiple of 3");
            return None;
        }

        // Now that we're in the vector of vertex indices, we want to look at the vertex
        // positions for each triangle, so we'll take indices in chunks of three, where each
        // chunk of three indices are references to the three vertices of a triangle.
        for index_chunk in indices.chunks_exact(3) {
            let [index1, index2, index3] = [
                usize::try_from(index_chunk[0]).ok()?,
                usize::try_from(index_chunk[1]).ok()?,
                usize::try_from(index_chunk[2]).ok()?,
            ];
            let triangle_index = Some(index1);
            let tri_vertex_positions = [
                Vec3A::from(vertex_positions[index1]),
                Vec3A::from(vertex_positions[index2]),
                Vec3A::from(vertex_positions[index3]),
            ];
            let tri_normals = vertex_normals.map(|normals| {
                [
                    Vec3A::from(normals[index1]),
                    Vec3A::from(normals[index2]),
                    Vec3A::from(normals[index3]),
                ]
            });

            let Some(hit) = triangle_intersection(
                tri_vertex_positions,
                tri_normals,
                closest_hit_distance,
                &mesh_space_ray,
                backface_culling,
            ) else {
                continue;
            };

            closest_hit = Some(RayMeshHit {
                point: mesh_transform.transform_point3(hit.point),
                normal: mesh_transform.transform_vector3(hit.normal),
                barycentric_coords: hit.barycentric_coords,
                distance: mesh_transform
                    .transform_vector3(mesh_space_ray.direction * hit.distance)
                    .length(),
                triangle: hit.triangle.map(|tri| {
                    [
                        mesh_transform.transform_point3a(tri[0]),
                        mesh_transform.transform_point3a(tri[1]),
                        mesh_transform.transform_point3a(tri[2]),
                    ]
                }),
                triangle_index,
            });
            closest_hit_distance = hit.distance;
        }
    } else {
        for (i, chunk) in vertex_positions.chunks_exact(3).enumerate() {
            let &[a, b, c] = chunk else {
                continue;
            };
            let triangle_index = Some(i);
            let tri_vertex_positions = [Vec3A::from(a), Vec3A::from(b), Vec3A::from(c)];
            let tri_normals = vertex_normals.map(|normals| {
                [
                    Vec3A::from(normals[i]),
                    Vec3A::from(normals[i + 1]),
                    Vec3A::from(normals[i + 2]),
                ]
            });

            let Some(hit) = triangle_intersection(
                tri_vertex_positions,
                tri_normals,
                closest_hit_distance,
                &mesh_space_ray,
                backface_culling,
            ) else {
                continue;
            };

            closest_hit = Some(RayMeshHit {
                point: mesh_transform.transform_point3(hit.point),
                normal: mesh_transform.transform_vector3(hit.normal),
                barycentric_coords: hit.barycentric_coords,
                distance: mesh_transform
                    .transform_vector3(mesh_space_ray.direction * hit.distance)
                    .length(),
                triangle: hit.triangle.map(|tri| {
                    [
                        mesh_transform.transform_point3a(tri[0]),
                        mesh_transform.transform_point3a(tri[1]),
                        mesh_transform.transform_point3a(tri[2]),
                    ]
                }),
                triangle_index,
            });
            closest_hit_distance = hit.distance;
        }
    }

    closest_hit
}

#[inline(always)]
fn triangle_intersection(
    tri_vertices: [Vec3A; 3],
    tri_normals: Option<[Vec3A; 3]>,
    max_distance: f32,
    ray: &Ray3d,
    backface_culling: Backfaces,
) -> Option<RayMeshHit> {
    let hit = ray_triangle_intersection(ray, &tri_vertices, backface_culling)?;

    if hit.distance < 0.0 || hit.distance > max_distance {
        return None;
    };

    let point = ray.get_point(hit.distance);
    let u = hit.barycentric_coords.0;
    let v = hit.barycentric_coords.1;
    let w = 1.0 - u - v;
    let barycentric = Vec3::new(u, v, w);

    let normal = if let Some(normals) = tri_normals {
        normals[1] * u + normals[2] * v + normals[0] * w
    } else {
        (tri_vertices[1] - tri_vertices[0])
            .cross(tri_vertices[2] - tri_vertices[0])
            .normalize()
    };

    Some(RayMeshHit {
        point,
        normal: normal.into(),
        barycentric_coords: barycentric,
        distance: hit.distance,
        triangle: Some(tri_vertices),
        triangle_index: None,
    })
}

/// Takes a ray and triangle and computes the intersection.
#[inline(always)]
fn ray_triangle_intersection(
    ray: &Ray3d,
    triangle: &[Vec3A; 3],
    backface_culling: Backfaces,
) -> Option<RayTriangleHit> {
    // Source: https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/moller-trumbore-ray-triangle-intersection
    let vector_v0_to_v1: Vec3A = triangle[1] - triangle[0];
    let vector_v0_to_v2: Vec3A = triangle[2] - triangle[0];
    let p_vec: Vec3A = (Vec3A::from(*ray.direction)).cross(vector_v0_to_v2);
    let determinant: f32 = vector_v0_to_v1.dot(p_vec);

    match backface_culling {
        Backfaces::Cull => {
            // if the determinant is negative the triangle is back facing
            // if the determinant is close to 0, the ray misses the triangle
            // This test checks both cases
            if determinant < f32::EPSILON {
                return None;
            }
        }
        Backfaces::Include => {
            // ray and triangle are parallel if det is close to 0
            if determinant.abs() < f32::EPSILON {
                return None;
            }
        }
    }

    let determinant_inverse = 1.0 / determinant;

    let t_vec = Vec3A::from(ray.origin) - triangle[0];
    let u = t_vec.dot(p_vec) * determinant_inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q_vec = t_vec.cross(vector_v0_to_v1);
    let v = Vec3A::from(*ray.direction).dot(q_vec) * determinant_inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    // The distance between ray origin and intersection is t.
    let t: f32 = vector_v0_to_v2.dot(q_vec) * determinant_inverse;

    Some(RayTriangleHit {
        distance: t,
        barycentric_coords: (u, v),
    })
}

// TODO: It'd be nice to use `RayCast3d` from `bevy_math` instead. It caches the direction reciprocal.
/// Checks if the ray intersects with an AABB of a mesh, returning `[near, far]` if it does.
pub(crate) fn ray_aabb_intersection_3d(
    ray: Ray3d,
    aabb: &Aabb,
    model_to_world: &Mat4,
) -> Option<[f32; 2]> {
    // Transform the ray to model space
    let world_to_model = model_to_world.inverse();
    let ray_dir: Vec3A = world_to_model.transform_vector3(*ray.direction).into();
    let ray_dir_recip = ray_dir.recip();
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

#[cfg(test)]
mod tests {
    use bevy_math::Vec3;

    use super::*;

    // Triangle vertices to be used in a left-hand coordinate system
    const V0: [f32; 3] = [1.0, -1.0, 2.0];
    const V1: [f32; 3] = [1.0, 2.0, -1.0];
    const V2: [f32; 3] = [1.0, -1.0, -1.0];

    #[test]
    fn ray_cast_triangle_mt() {
        let triangle = [V0.into(), V1.into(), V2.into()];
        let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
        let result = ray_triangle_intersection(&ray, &triangle, Backfaces::Include);
        assert!(result.unwrap().distance - 1.0 <= f32::EPSILON);
    }

    #[test]
    fn ray_cast_triangle_mt_culling() {
        let triangle = [V2.into(), V1.into(), V0.into()];
        let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
        let result = ray_triangle_intersection(&ray, &triangle, Backfaces::Cull);
        assert!(result.is_none());
    }
}
