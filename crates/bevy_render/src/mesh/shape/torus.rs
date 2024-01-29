use crate::{
    mesh::{Indices, Mesh},
    render_asset::RenderAssetPersistencePolicy,
};
use bevy_math::Vec3;
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

        let n_vertices = (torus.subdivisions_segments + 1) * (torus.subdivisions_sides + 1);
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(n_vertices);

        let segment_stride = 2.0 * std::f32::consts::PI / torus.subdivisions_segments as f32;
        let side_stride = 2.0 * std::f32::consts::PI / torus.subdivisions_sides as f32;

        for segment in 0..=torus.subdivisions_segments {
            let theta = segment_stride * segment as f32;

            for side in 0..=torus.subdivisions_sides {
                let phi = side_stride * side as f32;

                let position = Vec3::new(
                    theta.cos() * (torus.radius + torus.ring_radius * phi.cos()),
                    torus.ring_radius * phi.sin(),
                    theta.sin() * (torus.radius + torus.ring_radius * phi.cos()),
                );

                let center = Vec3::new(torus.radius * theta.cos(), 0., torus.radius * theta.sin());
                let normal = (position - center).normalize();

                positions.push(position.into());
                normals.push(normal.into());
                uvs.push([
                    segment as f32 / torus.subdivisions_segments as f32,
                    side as f32 / torus.subdivisions_sides as f32,
                ]);
            }
        }

        let n_faces = (torus.subdivisions_segments) * (torus.subdivisions_sides);
        let n_triangles = n_faces * 2;
        let n_indices = n_triangles * 3;

        let mut indices: Vec<u32> = Vec::with_capacity(n_indices);

        let n_vertices_per_row = torus.subdivisions_sides + 1;
        for segment in 0..torus.subdivisions_segments {
            for side in 0..torus.subdivisions_sides {
                let lt = side + segment * n_vertices_per_row;
                let rt = (side + 1) + segment * n_vertices_per_row;

                let lb = side + (segment + 1) * n_vertices_per_row;
                let rb = (side + 1) + (segment + 1) * n_vertices_per_row;

                indices.push(lt as u32);
                indices.push(rt as u32);
                indices.push(lb as u32);

                indices.push(rt as u32);
                indices.push(rb as u32);
                indices.push(lb as u32);
            }
        }

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetPersistencePolicy::Keep,
        )
        .with_indices(Some(Indices::U32(indices)))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}
