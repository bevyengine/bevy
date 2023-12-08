use super::Meshable;
use crate::mesh::{Indices, Mesh};
use bevy_math::{primitives::Torus, Vec3};
use wgpu::PrimitiveTopology;

#[derive(Clone, Copy, Debug)]
pub struct TorusMesh {
    pub torus: Torus,
    pub subdivisions_segments: usize,
    pub subdivisions_sides: usize,
}

impl Default for TorusMesh {
    fn default() -> Self {
        Self {
            torus: Torus::default(),
            subdivisions_segments: 32,
            subdivisions_sides: 24,
        }
    }
}

impl TorusMesh {
    pub fn build(&self) -> Mesh {
        // code adapted from http://apparat-engine.blogspot.com/2013/04/procedural-meshes-torus.html
        // (source code at https://github.com/SEilers/Apparat)

        let n_vertices = (self.subdivisions_segments + 1) * (self.subdivisions_sides + 1);
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(n_vertices);

        let segment_stride = 2.0 * std::f32::consts::PI / self.subdivisions_segments as f32;
        let side_stride = 2.0 * std::f32::consts::PI / self.subdivisions_sides as f32;

        for segment in 0..=self.subdivisions_segments {
            let theta = segment_stride * segment as f32;

            for side in 0..=self.subdivisions_sides {
                let phi = side_stride * side as f32;

                let position = Vec3::new(
                    theta.cos() * (self.torus.major_radius + self.torus.minor_radius * phi.cos()),
                    self.torus.minor_radius * phi.sin(),
                    theta.sin() * (self.torus.major_radius + self.torus.minor_radius * phi.cos()),
                );

                let center = Vec3::new(
                    self.torus.major_radius * theta.cos(),
                    0.,
                    self.torus.major_radius * theta.sin(),
                );
                let normal = (position - center).normalize();

                positions.push(position.into());
                normals.push(normal.into());
                uvs.push([
                    segment as f32 / self.subdivisions_segments as f32,
                    side as f32 / self.subdivisions_sides as f32,
                ]);
            }
        }

        let n_faces = (self.subdivisions_segments) * (self.subdivisions_sides);
        let n_triangles = n_faces * 2;
        let n_indices = n_triangles * 3;

        let mut indices: Vec<u32> = Vec::with_capacity(n_indices);

        let n_vertices_per_row = self.subdivisions_sides + 1;
        for segment in 0..self.subdivisions_segments {
            for side in 0..self.subdivisions_sides {
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

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_indices(Some(Indices::U32(indices)))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }

    pub fn subdivisions_segments(mut self, subdivisions: usize) -> Self {
        self.subdivisions_segments = subdivisions;
        self
    }

    pub fn subdivisions_sides(mut self, subdivisions: usize) -> Self {
        self.subdivisions_sides = subdivisions;
        self
    }
}

impl Meshable for Torus {
    type Output = TorusMesh;

    fn mesh(&self) -> Self::Output {
        TorusMesh {
            torus: *self,
            ..Default::default()
        }
    }
}

impl From<Torus> for Mesh {
    fn from(torus: Torus) -> Self {
        torus.mesh().build()
    }
}

impl From<TorusMesh> for Mesh {
    fn from(torus: TorusMesh) -> Self {
        torus.build()
    }
}
