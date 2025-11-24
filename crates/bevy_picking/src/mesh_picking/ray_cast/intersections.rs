use bevy_math::{bounding::Aabb3d, Affine3A, Dir3, Ray3d, Vec2, Vec3, Vec3A};
use bevy_mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};
use bevy_reflect::Reflect;

use super::Backfaces;

/// Hit data for an intersection between a ray and a mesh.
#[derive(Debug, Clone, Reflect)]
#[reflect(Clone)]
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
    /// UV coordinate of the hit, if the mesh has UV attributes.
    pub uv: Option<Vec2>,
    /// The index of the triangle that was hit.
    pub triangle_index: Option<usize>,
}

/// Hit data for an intersection between a ray and a triangle.
#[derive(Default, Debug)]
pub struct RayTriangleHit {
    pub distance: f32,
    /// Note this uses the convention from the Moller-Trumbore algorithm:
    /// P = (1 - u - v)A + uB + vC
    /// This is different from the more common convention of
    /// P = uA + vB + (1 - u - v)C
    pub barycentric_coords: (f32, f32),
}

/// Casts a ray on a mesh, and returns the intersection.
pub(super) fn ray_intersection_over_mesh(
    mesh: &Mesh,
    transform: &Affine3A,
    ray: Ray3d,
    cull: Backfaces,
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

    let uvs = mesh
        .attribute(Mesh::ATTRIBUTE_UV_0)
        .and_then(|uvs| match uvs {
            VertexAttributeValues::Float32x2(uvs) => Some(uvs.as_slice()),
            _ => None,
        });

    match mesh.indices() {
        Some(Indices::U16(indices)) => {
            ray_mesh_intersection(ray, transform, positions, normals, Some(indices), uvs, cull)
        }
        Some(Indices::U32(indices)) => {
            ray_mesh_intersection(ray, transform, positions, normals, Some(indices), uvs, cull)
        }
        None => ray_mesh_intersection::<usize>(ray, transform, positions, normals, None, uvs, cull),
    }
}

/// Checks if a ray intersects a mesh, and returns the nearest intersection if one exists.
pub fn ray_mesh_intersection<I>(
    ray: Ray3d,
    mesh_transform: &Affine3A,
    positions: &[[f32; 3]],
    vertex_normals: Option<&[[f32; 3]]>,
    indices: Option<&[I]>,
    uvs: Option<&[[f32; 2]]>,
    backface_culling: Backfaces,
) -> Option<RayMeshHit>
where
    I: TryInto<usize> + Clone + Copy,
{
    let world_to_mesh = mesh_transform.inverse();

    let ray = Ray3d::new(
        world_to_mesh.transform_point3(ray.origin),
        Dir3::new(world_to_mesh.transform_vector3(*ray.direction)).ok()?,
    );

    let closest_hit = if let Some(indices) = indices {
        // The index list must be a multiple of three. If not, the mesh is malformed and the raycast
        // result might be nonsensical.
        if indices.len() % 3 != 0 {
            return None;
        }

        indices
            .chunks_exact(3)
            .enumerate()
            .fold(
                (f32::MAX, None),
                |(closest_distance, closest_hit), (tri_idx, triangle)| {
                    let [Ok(a), Ok(b), Ok(c)] = [
                        triangle[0].try_into(),
                        triangle[1].try_into(),
                        triangle[2].try_into(),
                    ] else {
                        return (closest_distance, closest_hit);
                    };

                    let tri_vertices = match [positions.get(a), positions.get(b), positions.get(c)]
                    {
                        [Some(a), Some(b), Some(c)] => {
                            [Vec3::from(*a), Vec3::from(*b), Vec3::from(*c)]
                        }
                        _ => return (closest_distance, closest_hit),
                    };

                    match ray_triangle_intersection(&ray, &tri_vertices, backface_culling) {
                        Some(hit) if hit.distance >= 0. && hit.distance < closest_distance => {
                            (hit.distance, Some((tri_idx, hit)))
                        }
                        _ => (closest_distance, closest_hit),
                    }
                },
            )
            .1
    } else {
        positions
            .chunks_exact(3)
            .enumerate()
            .fold(
                (f32::MAX, None),
                |(closest_distance, closest_hit), (tri_idx, triangle)| {
                    let tri_vertices = [
                        Vec3::from(triangle[0]),
                        Vec3::from(triangle[1]),
                        Vec3::from(triangle[2]),
                    ];

                    match ray_triangle_intersection(&ray, &tri_vertices, backface_culling) {
                        Some(hit) if hit.distance >= 0. && hit.distance < closest_distance => {
                            (hit.distance, Some((tri_idx, hit)))
                        }
                        _ => (closest_distance, closest_hit),
                    }
                },
            )
            .1
    };

    closest_hit.and_then(|(tri_idx, hit)| {
        let [a, b, c] = match indices {
            Some(indices) => {
                let [i, j, k] = [tri_idx * 3, tri_idx * 3 + 1, tri_idx * 3 + 2];
                [
                    indices.get(i).copied()?.try_into().ok()?,
                    indices.get(j).copied()?.try_into().ok()?,
                    indices.get(k).copied()?.try_into().ok()?,
                ]
            }
            None => [tri_idx * 3, tri_idx * 3 + 1, tri_idx * 3 + 2],
        };

        let tri_vertices = match [positions.get(a), positions.get(b), positions.get(c)] {
            [Some(a), Some(b), Some(c)] => [Vec3::from(*a), Vec3::from(*b), Vec3::from(*c)],
            _ => return None,
        };

        let tri_normals = vertex_normals.and_then(|normals| {
            let [Some(a), Some(b), Some(c)] = [normals.get(a), normals.get(b), normals.get(c)]
            else {
                return None;
            };
            Some([Vec3::from(*a), Vec3::from(*b), Vec3::from(*c)])
        });

        let point = ray.get_point(hit.distance);
        // Note that we need to convert from the Möller-Trumbore convention to the more common
        // P = uA + vB + (1 - u - v)C convention.
        let u = hit.barycentric_coords.0;
        let v = hit.barycentric_coords.1;
        let w = 1.0 - u - v;
        let barycentric = Vec3::new(w, u, v);

        let normal = if let Some(normals) = tri_normals {
            normals[1] * u + normals[2] * v + normals[0] * w
        } else {
            (tri_vertices[1] - tri_vertices[0])
                .cross(tri_vertices[2] - tri_vertices[0])
                .normalize()
        };

        let uv = uvs.and_then(|uvs| {
            let tri_uvs = if let Some(indices) = indices {
                let i = tri_idx * 3;
                [
                    uvs[indices[i].try_into().ok()?],
                    uvs[indices[i + 1].try_into().ok()?],
                    uvs[indices[i + 2].try_into().ok()?],
                ]
            } else {
                let i = tri_idx * 3;
                [uvs[i], uvs[i + 1], uvs[i + 2]]
            };
            Some(
                barycentric.x * Vec2::from(tri_uvs[0])
                    + barycentric.y * Vec2::from(tri_uvs[1])
                    + barycentric.z * Vec2::from(tri_uvs[2]),
            )
        });

        Some(RayMeshHit {
            point: mesh_transform.transform_point3(point),
            normal: mesh_transform.transform_vector3(normal),
            uv,
            barycentric_coords: barycentric,
            distance: mesh_transform
                .transform_vector3(ray.direction * hit.distance)
                .length(),
            triangle: Some(tri_vertices.map(|v| mesh_transform.transform_point3(v))),
            triangle_index: Some(tri_idx),
        })
    })
}

/// Takes a ray and triangle and computes the intersection.
#[inline]
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
pub fn ray_aabb_intersection_3d(
    ray: Ray3d,
    aabb: &Aabb3d,
    model_to_world: &Affine3A,
) -> Option<f32> {
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
    use bevy_transform::components::GlobalTransform;

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

    #[test]
    fn ray_mesh_intersection_simple() {
        let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
        let mesh_transform = GlobalTransform::IDENTITY.affine();
        let positions = &[V0, V1, V2];
        let vertex_normals = None;
        let indices: Option<&[u16]> = None;
        let backface_culling = Backfaces::Cull;

        let result = ray_mesh_intersection(
            ray,
            &mesh_transform,
            positions,
            vertex_normals,
            indices,
            None,
            backface_culling,
        );

        assert!(result.is_some());
    }

    #[test]
    fn ray_mesh_intersection_indices() {
        let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
        let mesh_transform = GlobalTransform::IDENTITY.affine();
        let positions = &[V0, V1, V2];
        let vertex_normals = None;
        let indices: Option<&[u16]> = Some(&[0, 1, 2]);
        let backface_culling = Backfaces::Cull;

        let result = ray_mesh_intersection(
            ray,
            &mesh_transform,
            positions,
            vertex_normals,
            indices,
            None,
            backface_culling,
        );

        assert!(result.is_some());
    }

    #[test]
    fn ray_mesh_intersection_indices_vertex_normals() {
        let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
        let mesh_transform = GlobalTransform::IDENTITY.affine();
        let positions = &[V0, V1, V2];
        let vertex_normals: Option<&[[f32; 3]]> =
            Some(&[[-1., 0., 0.], [-1., 0., 0.], [-1., 0., 0.]]);
        let indices: Option<&[u16]> = Some(&[0, 1, 2]);
        let backface_culling = Backfaces::Cull;

        let result = ray_mesh_intersection(
            ray,
            &mesh_transform,
            positions,
            vertex_normals,
            indices,
            None,
            backface_culling,
        );

        assert!(result.is_some());
    }

    #[test]
    fn ray_mesh_intersection_vertex_normals() {
        let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
        let mesh_transform = GlobalTransform::IDENTITY.affine();
        let positions = &[V0, V1, V2];
        let vertex_normals: Option<&[[f32; 3]]> =
            Some(&[[-1., 0., 0.], [-1., 0., 0.], [-1., 0., 0.]]);
        let indices: Option<&[u16]> = None;
        let backface_culling = Backfaces::Cull;

        let result = ray_mesh_intersection(
            ray,
            &mesh_transform,
            positions,
            vertex_normals,
            indices,
            None,
            backface_culling,
        );

        assert!(result.is_some());
    }

    #[test]
    fn ray_mesh_intersection_missing_vertex_normals() {
        let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
        let mesh_transform = GlobalTransform::IDENTITY.affine();
        let positions = &[V0, V1, V2];
        let vertex_normals: Option<&[[f32; 3]]> = Some(&[]);
        let indices: Option<&[u16]> = None;
        let backface_culling = Backfaces::Cull;

        let result = ray_mesh_intersection(
            ray,
            &mesh_transform,
            positions,
            vertex_normals,
            indices,
            None,
            backface_culling,
        );

        assert!(result.is_some());
    }

    #[test]
    fn ray_mesh_intersection_indices_missing_vertex_normals() {
        let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
        let mesh_transform = GlobalTransform::IDENTITY.affine();
        let positions = &[V0, V1, V2];
        let vertex_normals: Option<&[[f32; 3]]> = Some(&[]);
        let indices: Option<&[u16]> = Some(&[0, 1, 2]);
        let backface_culling = Backfaces::Cull;

        let result = ray_mesh_intersection(
            ray,
            &mesh_transform,
            positions,
            vertex_normals,
            indices,
            None,
            backface_culling,
        );

        assert!(result.is_some());
    }

    #[test]
    fn ray_mesh_intersection_not_enough_indices() {
        let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
        let mesh_transform = GlobalTransform::IDENTITY.affine();
        let positions = &[V0, V1, V2];
        let vertex_normals = None;
        let indices: Option<&[u16]> = Some(&[0]);
        let backface_culling = Backfaces::Cull;

        let result = ray_mesh_intersection(
            ray,
            &mesh_transform,
            positions,
            vertex_normals,
            indices,
            None,
            backface_culling,
        );

        assert!(result.is_none());
    }

    #[test]
    fn ray_mesh_intersection_bad_indices() {
        let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
        let mesh_transform = GlobalTransform::IDENTITY.affine();
        let positions = &[V0, V1, V2];
        let vertex_normals = None;
        let indices: Option<&[u16]> = Some(&[0, 1, 3]);
        let backface_culling = Backfaces::Cull;

        let result = ray_mesh_intersection(
            ray,
            &mesh_transform,
            positions,
            vertex_normals,
            indices,
            None,
            backface_culling,
        );

        assert!(result.is_none());
    }
}
