//! Ray casting on meshes.

mod immediate;
mod intersections;
mod simplified_mesh;

pub use immediate::*;
pub use simplified_mesh::*;

use bevy_math::{Mat4, Ray3d, Vec3, Vec3A};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    mesh::{Indices, Mesh, VertexAttributeValues},
    render_resource::PrimitiveTopology,
};
use bevy_utils::tracing::{error, warn};

use intersections::*;

/// Casts a ray on a mesh, and returns the intersection
pub fn ray_intersection_over_mesh(
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
    // Get the vertex positions from the mesh reference resolved from the mesh handle
    let vertex_positions: &Vec<[f32; 3]> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        None => panic!("Mesh does not contain vertex positions"),
        Some(vertex_values) => match &vertex_values {
            VertexAttributeValues::Float32x3(positions) => positions,
            _ => panic!("Unexpected types in {:?}", Mesh::ATTRIBUTE_POSITION),
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
        // Iterate over the list of pick rays that belong to the same group as this mesh
        match indices {
            Indices::U16(vertex_indices) => ray_mesh_intersection(
                mesh_transform,
                vertex_positions,
                vertex_normals,
                ray,
                Some(vertex_indices),
                backface_culling,
            ),
            Indices::U32(vertex_indices) => ray_mesh_intersection(
                mesh_transform,
                vertex_positions,
                vertex_normals,
                ray,
                Some(vertex_indices),
                backface_culling,
            ),
        }
    } else {
        ray_mesh_intersection(
            mesh_transform,
            vertex_positions,
            vertex_normals,
            ray,
            None::<&Vec<u32>>,
            backface_culling,
        )
    }
}

/// A trait for converting a value into a [`usize`].
pub trait IntoUsize: Copy {
    /// Converts the value into a [`usize`].
    fn into_usize(self) -> usize;
}

impl IntoUsize for u16 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

impl IntoUsize for u32 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

/// Checks if a ray intersects a mesh, and returns the nearest intersection if one exists.
pub fn ray_mesh_intersection(
    mesh_transform: &Mat4,
    vertex_positions: &[[f32; 3]],
    vertex_normals: Option<&[[f32; 3]]>,
    ray: Ray3d,
    indices: Option<&Vec<impl IntoUsize>>,
    backface_culling: Backfaces,
) -> Option<RayMeshHit> {
    // The ray cast can hit the same mesh many times, so we need to track which hit is
    // closest to the camera, and record that.
    let mut min_pick_distance = f32::MAX;
    let mut pick_intersection = None;

    let world_to_mesh = mesh_transform.inverse();

    let mesh_space_ray = Ray3d::new(
        world_to_mesh.transform_point3(ray.origin),
        world_to_mesh.transform_vector3(*ray.direction),
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
        for index in indices.chunks(3) {
            let triangle_index = Some(index[0].into_usize());
            let tri_vertex_positions = [
                Vec3A::from(vertex_positions[index[0].into_usize()]),
                Vec3A::from(vertex_positions[index[1].into_usize()]),
                Vec3A::from(vertex_positions[index[2].into_usize()]),
            ];
            let tri_normals = vertex_normals.map(|normals| {
                [
                    Vec3A::from(normals[index[0].into_usize()]),
                    Vec3A::from(normals[index[1].into_usize()]),
                    Vec3A::from(normals[index[2].into_usize()]),
                ]
            });
            let intersection = triangle_intersection(
                tri_vertex_positions,
                tri_normals,
                min_pick_distance,
                &mesh_space_ray,
                backface_culling,
            );
            if let Some(i) = intersection {
                pick_intersection = Some(RayMeshHit::new(
                    mesh_transform.transform_point3(i.position()),
                    mesh_transform.transform_vector3(i.normal()),
                    i.barycentric_coord(),
                    mesh_transform
                        .transform_vector3(mesh_space_ray.direction * i.distance())
                        .length(),
                    i.triangle().map(|tri| {
                        [
                            mesh_transform.transform_point3a(tri[0]),
                            mesh_transform.transform_point3a(tri[1]),
                            mesh_transform.transform_point3a(tri[2]),
                        ]
                    }),
                    triangle_index,
                ));
                min_pick_distance = i.distance();
            }
        }
    } else {
        for i in (0..vertex_positions.len()).step_by(3) {
            let triangle_index = Some(i);
            let tri_vertex_positions = [
                Vec3A::from(vertex_positions[i]),
                Vec3A::from(vertex_positions[i + 1]),
                Vec3A::from(vertex_positions[i + 2]),
            ];
            let tri_normals = vertex_normals.map(|normals| {
                [
                    Vec3A::from(normals[i]),
                    Vec3A::from(normals[i + 1]),
                    Vec3A::from(normals[i + 2]),
                ]
            });
            let intersection = triangle_intersection(
                tri_vertex_positions,
                tri_normals,
                min_pick_distance,
                &mesh_space_ray,
                backface_culling,
            );
            if let Some(i) = intersection {
                pick_intersection = Some(RayMeshHit::new(
                    mesh_transform.transform_point3(i.position()),
                    mesh_transform.transform_vector3(i.normal()),
                    i.barycentric_coord(),
                    mesh_transform
                        .transform_vector3(mesh_space_ray.direction * i.distance())
                        .length(),
                    i.triangle().map(|tri| {
                        [
                            mesh_transform.transform_point3a(tri[0]),
                            mesh_transform.transform_point3a(tri[1]),
                            mesh_transform.transform_point3a(tri[2]),
                        ]
                    }),
                    triangle_index,
                ));
                min_pick_distance = i.distance();
            }
        }
    }
    pick_intersection
}

#[inline(always)]
fn triangle_intersection(
    tri_vertices: [Vec3A; 3],
    tri_normals: Option<[Vec3A; 3]>,
    max_distance: f32,
    ray: &Ray3d,
    backface_culling: Backfaces,
) -> Option<RayMeshHit> {
    // Run the ray cast on the ray and triangle
    let ray_hit = ray_triangle_intersection(ray, &tri_vertices, backface_culling)?;
    let distance = *ray_hit.distance();
    if distance < 0.0 || distance > max_distance {
        return None;
    };
    let position = ray.get_point(distance);
    let u = ray_hit.uv_coords().0;
    let v = ray_hit.uv_coords().1;
    let w = 1.0 - u - v;
    let barycentric = Vec3::new(u, v, w);
    let normal = if let Some(normals) = tri_normals {
        normals[1] * u + normals[2] * v + normals[0] * w
    } else {
        (tri_vertices[1] - tri_vertices[0])
            .cross(tri_vertices[2] - tri_vertices[0])
            .normalize()
    };
    Some(RayMeshHit::new(
        position,
        normal.into(),
        barycentric,
        distance,
        Some(tri_vertices),
        None,
    ))
}

/// Determines whether backfaces should be culled or included in intersection checks.
#[derive(Copy, Clone, Default, Reflect)]
#[reflect(Default)]
pub enum Backfaces {
    /// Cull backfaces.
    #[default]
    Cull,
    /// Include backfaces.
    Include,
}

/// Takes a ray and triangle and computes the intersection and normal
#[inline(always)]
pub fn ray_triangle_intersection(
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
        uv_coords: (u, v),
    })
}

/// A hit result from a ray cast on a triangle.
#[derive(Default, Debug)]
pub struct RayTriangleHit {
    distance: f32,
    uv_coords: (f32, f32),
}

impl RayTriangleHit {
    /// Get a reference to the intersection's uv coords.
    pub fn uv_coords(&self) -> &(f32, f32) {
        &self.uv_coords
    }

    /// Get a reference to the intersection's distance.
    pub fn distance(&self) -> &f32 {
        &self.distance
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
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let result = ray_triangle_intersection(&ray, &triangle, Backfaces::Include);
        assert!(result.unwrap().distance - 1.0 <= f32::EPSILON);
    }

    #[test]
    fn ray_cast_triangle_mt_culling() {
        let triangle = [V2.into(), V1.into(), V0.into()];
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let result = ray_triangle_intersection(&ray, &triangle, Backfaces::Cull);
        assert!(result.is_none());
    }
}
