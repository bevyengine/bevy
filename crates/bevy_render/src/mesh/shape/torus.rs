use crate::{
    mesh::{Indices, Mesh},
    pipeline::PrimitiveTopology,
};
use bevy_math::{Quat, Vec3};

/// A torus (donut) shape
#[derive(Debug)]
pub struct Torus {
    pub radius: f32,
    pub tube_radius: f32,
    pub subdivisions_segments: usize,
    pub subdivisions_sides: usize,
}

impl Default for Torus {
    fn default() -> Self {
        Torus {
            radius: 1.0,
            tube_radius: 0.5,
            subdivisions_segments: 32,
            subdivisions_sides: 24,
        }
    }
}

impl From<Torus> for Mesh {
    fn from(torus: Torus) -> Self {
        // code adapted from http://wiki.unity3d.com/index.php/ProceduralPrimitives#C.23_-_Torus

        let n_vertices = (torus.subdivisions_segments + 1) * (torus.subdivisions_sides + 1);
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut uvs: Vec<[f32; 2]> = Vec::new();

        for segment in 0..=torus.subdivisions_segments {
            let t1 =
                segment as f32 / torus.subdivisions_segments as f32 * 2.0 * std::f32::consts::PI;
            let r1 = Vec3::new(t1.cos() * torus.radius, 0.0, t1.sin() * torus.radius);

            for side in 0..=torus.subdivisions_sides {
                let t2 = side as f32 / torus.subdivisions_sides as f32 * 2.0 * std::f32::consts::PI;
                let r2 = Quat::from_axis_angle(Vec3::unit_y(), -t1)
                    * Vec3::new(
                        t2.sin() * torus.tube_radius,
                        t2.cos() * torus.tube_radius,
                        0.0,
                    );

                let position = r1 + r2;
                let normal = r1.cross(Vec3::unit_y()).normalize();
                let uv = [
                    segment as f32 / torus.subdivisions_segments as f32,
                    side as f32 / torus.subdivisions_sides as f32,
                ];

                positions.push(position.into());
                normals.push(normal.into());
                uvs.push(uv);
            }
        }

        let n_faces = (torus.subdivisions_segments + 1) * (torus.subdivisions_sides);
        let n_triangles = n_faces * 2;
        let n_indices = n_triangles * 3;

        let mut indices: Vec<u32> = Vec::with_capacity(n_indices);

        for segment in 0..=torus.subdivisions_segments as u32 {
            for side in 0..torus.subdivisions_sides as u32 {
                let current = side + segment * (torus.subdivisions_sides as u32 + 1);

                let next = if segment < torus.subdivisions_segments as u32 {
                    (segment + 1) * (torus.subdivisions_sides as u32 + 1)
                } else {
                    0
                } + side;

                indices.push(current);
                indices.push(next);
                indices.push(next + 1);

                indices.push(current);
                indices.push(next + 1);
                indices.push(current + 1);
            }
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
    }
}
