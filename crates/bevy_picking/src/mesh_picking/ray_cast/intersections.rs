use bevy_math::{bounding::Aabb3d, Dir3, Mat4, Ray3d, Vec3, Vec3A};
use bevy_reflect::Reflect;
use bevy_render::mesh::{Indices, Mesh, PrimitiveTopology};

use super::Backfaces;

/// Hit data for an intersection between a ray and a mesh.
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
    pub triangle: Option<[Vec3; 3]>,
    /// The index of the triangle that was hit.
    pub triangle_index: Option<usize>,
}

/// Hit data for an intersection between a ray and a triangle.
#[derive(Default, Debug)]
pub struct RayTriangleHit {
    pub distance: f32,
    pub barycentric_coords: (f32, f32),
}

/// Casts a ray on a mesh, and returns the intersection.
pub(super) fn ray_intersection_over_mesh(
    mesh: &Mesh,
    transform: &Mat4,
    ray: Ray3d,
    culling: Backfaces,
) -> Option<RayMeshHit> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        return None; // ray_mesh_intersection assumes vertices are laid out in a triangle list
    }
    // Vertex positions are required
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;

    // Normals are optional
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .and_then(|normal_values| normal_values.as_float3());

    match mesh.indices() {
        Some(Indices::U16(indices)) => {
            ray_mesh_intersection(ray, transform, positions, normals, Some(indices), culling)
        }
        Some(Indices::U32(indices)) => {
            ray_mesh_intersection(ray, transform, positions, normals, Some(indices), culling)
        }
        None => ray_mesh_intersection::<usize>(ray, transform, positions, normals, None, culling),
    }
}

/// Checks if a ray intersects a mesh, and returns the nearest intersection if one exists.
pub fn ray_mesh_intersection<I: TryInto<usize> + Clone + Copy>(
    ray: Ray3d,
    mesh_transform: &Mat4,
    positions: &[[f32; 3]],
    vertex_normals: Option<&[[f32; 3]]>,
    indices: Option<&[I]>,
    backface_culling: Backfaces,
) -> Option<RayMeshHit> {
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
        // The index list must be a multiple of three. If not, the mesh is malformed and the raycast
        // result might be nonsensical.
        if indices.len() % 3 != 0 {
            return None;
        }

        for triangle in indices.chunks_exact(3) {
            let [a, b, c] = [
                triangle[0].try_into().ok()?,
                triangle[1].try_into().ok()?,
                triangle[2].try_into().ok()?,
            ];

            let triangle_index = Some(a);
            let tri_vertex_positions = &[
                Vec3::from(positions[a]),
                Vec3::from(positions[b]),
                Vec3::from(positions[c]),
            ];
            let tri_normals = vertex_normals.map(|normals| {
                [
                    Vec3::from(normals[a]),
                    Vec3::from(normals[b]),
                    Vec3::from(normals[c]),
                ]
            });

            let Some(hit) = triangle_intersection(
                tri_vertex_positions,
                tri_normals.as_ref(),
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
                        mesh_transform.transform_point3(tri[0]),
                        mesh_transform.transform_point3(tri[1]),
                        mesh_transform.transform_point3(tri[2]),
                    ]
                }),
                triangle_index,
            });
            closest_hit_distance = hit.distance;
        }
    } else {
        for (i, triangle) in positions.chunks_exact(3).enumerate() {
            let &[a, b, c] = triangle else {
                continue;
            };
            let triangle_index = Some(i);
            let tri_vertex_positions = &[Vec3::from(a), Vec3::from(b), Vec3::from(c)];
            let tri_normals = vertex_normals.map(|normals| {
                [
                    Vec3::from(normals[i]),
                    Vec3::from(normals[i + 1]),
                    Vec3::from(normals[i + 2]),
                ]
            });

            let Some(hit) = triangle_intersection(
                tri_vertex_positions,
                tri_normals.as_ref(),
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
                        mesh_transform.transform_point3(tri[0]),
                        mesh_transform.transform_point3(tri[1]),
                        mesh_transform.transform_point3(tri[2]),
                    ]
                }),
                triangle_index,
            });
            closest_hit_distance = hit.distance;
        }
    }

    closest_hit
}

fn triangle_intersection(
    tri_vertices: &[Vec3; 3],
    tri_normals: Option<&[Vec3; 3]>,
    max_distance: f32,
    ray: &Ray3d,
    backface_culling: Backfaces,
) -> Option<RayMeshHit> {
    let hit = ray_triangle_intersection(ray, tri_vertices, backface_culling)?;

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
        normal,
        barycentric_coords: barycentric,
        distance: hit.distance,
        triangle: Some(*tri_vertices),
        triangle_index: None,
    })
}

/// Takes a ray and triangle and computes the intersection.
fn ray_triangle_intersection(
    ray: &Ray3d,
    triangle: &[Vec3; 3],
    backface_culling: Backfaces,
) -> Option<RayTriangleHit> {
    // Source: https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/moller-trumbore-ray-triangle-intersection
    let vector_v0_to_v1: Vec3 = triangle[1] - triangle[0];
    let vector_v0_to_v2: Vec3 = triangle[2] - triangle[0];
    let p_vec: Vec3 = ray.direction.cross(vector_v0_to_v2);
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

    let t_vec = ray.origin - triangle[0];
    let u = t_vec.dot(p_vec) * determinant_inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q_vec = t_vec.cross(vector_v0_to_v1);
    let v = (*ray.direction).dot(q_vec) * determinant_inverse;
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

// TODO: It'd be nice to reuse `RayCast3d::aabb_intersection_at`, but it assumes a normalized ray.
//       In our case, the ray is transformed to model space, which could involve scaling.
/// Checks if the ray intersects with the AABB of a mesh, returning the distance to the point of intersection.
/// The distance is zero if the ray starts inside the AABB.
pub fn ray_aabb_intersection_3d(ray: Ray3d, aabb: &Aabb3d, model_to_world: &Mat4) -> Option<f32> {
    // Transform the ray to model space
    let world_to_model = model_to_world.inverse();
    let ray_direction: Vec3A = world_to_model.transform_vector3a((*ray.direction).into());
    let ray_direction_recip = ray_direction.recip();
    let ray_origin: Vec3A = world_to_model.transform_point3a(ray.origin.into());

    // Check if the ray intersects the mesh's AABB. It's useful to work in model space
    // because we can do an AABB intersection test, instead of an OBB intersection test.

    // NOTE: This is largely copied from `RayCast3d::aabb_intersection_at`.
    let positive = ray_direction.signum().cmpgt(Vec3A::ZERO);
    let min = Vec3A::select(positive, aabb.min, aabb.max);
    let max = Vec3A::select(positive, aabb.max, aabb.min);

    // Calculate the minimum/maximum time for each axis based on how much the direction goes that
    // way. These values can get arbitrarily large, or even become NaN, which is handled by the
    // min/max operations below
    let tmin = (min - ray_origin) * ray_direction_recip;
    let tmax = (max - ray_origin) * ray_direction_recip;

    // An axis that is not relevant to the ray direction will be NaN. When one of the arguments
    // to min/max is NaN, the other argument is used.
    // An axis for which the direction is the wrong way will return an arbitrarily large
    // negative value.
    let tmin = tmin.max_element().max(0.0);
    let tmax = tmax.min_element();

    if tmin <= tmax {
        Some(tmin)
    } else {
        None
    }
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
