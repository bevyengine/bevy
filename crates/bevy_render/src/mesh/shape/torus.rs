use crate::mesh::{Indices, Mesh};
use bevy_math::Vec3;
use itertools::Itertools;
use wgpu::PrimitiveTopology;

/// A torus (donut) shape.
#[derive(Debug, Clone, Copy)]
pub struct Torus {
    pub radius: f32,
    pub ring_radius: f32,
    pub subdivisions_segments: usize,
    pub subdivisions_sides: usize,
}

impl Default for Torus {
    fn default() -> Self {
        Torus {
            radius: 1.0,
            ring_radius: 0.5,
            subdivisions_segments: 32,
            subdivisions_sides: 24,
        }
    }
}

impl From<Torus> for Mesh {
    fn from(torus: Torus) -> Self {
        // code adapted from http://apparat-engine.blogspot.com/2013/04/procedural-meshes-torus.html
        // (source code at https://github.com/SEilers/Apparat)

        let segment_stride = 2.0 * std::f32::consts::PI / torus.subdivisions_segments as f32;
        let side_stride = 2.0 * std::f32::consts::PI / torus.subdivisions_sides as f32;

        let positions: Vec<[f32; 3]>;
        let normals: Vec<[f32; 3]>;
        let uvs: Vec<[f32; 2]>;

        (positions, (normals, uvs)) = (0..=torus.subdivisions_segments)
            .map(|segment| {
                let theta = segment_stride * segment as f32;
                (segment, theta)
            })
            .cartesian_product(0..torus.subdivisions_sides)
            .map(|((segment, theta), side)| {
                let phi = side_stride * side as f32;

                let position = Vec3::new(
                    theta.cos() * (torus.radius + torus.ring_radius * phi.cos()),
                    torus.ring_radius * phi.sin(),
                    theta.sin() * (torus.radius + torus.ring_radius * phi.cos()),
                );

                let center = Vec3::new(torus.radius * theta.cos(), 0., torus.radius * theta.sin());
                let normal = (position - center).normalize();

                let uv = [
                    segment as f32 / torus.subdivisions_segments as f32,
                    side as f32 / torus.subdivisions_sides as f32,
                ];

                (position.to_array(), (normal.to_array(), uv))
            })
            .unzip();

        let n_vertices_per_row = torus.subdivisions_sides + 1;
        let indices = (0..torus.subdivisions_segments)
            .cartesian_product(0..torus.subdivisions_sides)
            .flat_map(|(segment, side)| {
                let lt = (side + segment * n_vertices_per_row) as u32;
                let rt = ((side + 1) + segment * n_vertices_per_row) as u32;

                let lb = (side + (segment + 1) * n_vertices_per_row) as u32;
                let rb = ((side + 1) + (segment + 1) * n_vertices_per_row) as u32;

                [lt, rt, lb, rt, rb, lb]
            })
            .collect();

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
    }
}
