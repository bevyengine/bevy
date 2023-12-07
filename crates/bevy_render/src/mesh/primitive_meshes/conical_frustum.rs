use super::Meshable;
use crate::mesh::{Indices, Mesh};
use bevy_math::{primitives::ConicalFrustum, Vec3};
use wgpu::PrimitiveTopology;

pub struct ConicalFrustumBuilder {
    pub frustum: ConicalFrustum,
    pub resolution: u32,
    pub segments: u32,
}

impl ConicalFrustumBuilder {
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    pub fn segments(mut self, segments: u32) -> Self {
        self.segments = segments;
        self
    }

    pub fn build(&self) -> Mesh {
        debug_assert!(self.resolution > 2);
        debug_assert!(self.segments > 0);

        let ConicalFrustum {
            radius_top,
            radius_bottom,
            height,
        } = self.frustum;

        let num_rings = self.segments + 1;
        let num_vertices = self.resolution * 2 + num_rings * (self.resolution + 1);
        let num_faces = self.resolution * (num_rings - 2);
        let num_indices = (2 * num_faces + 2 * (self.resolution - 1) * 2) * 3;

        let mut positions = Vec::with_capacity(num_vertices as usize);
        let mut normals = Vec::with_capacity(num_vertices as usize);
        let mut uvs = Vec::with_capacity(num_vertices as usize);
        let mut indices = Vec::with_capacity(num_indices as usize);

        let step_theta = std::f32::consts::TAU / self.resolution as f32;
        let step_y = height / self.segments as f32;
        let step_radius = (radius_top - radius_bottom) / self.segments as f32;

        // rings

        for ring in 0..num_rings {
            let y = -height / 2.0 + ring as f32 * step_y;
            let radius = radius_bottom + ring as f32 * step_radius;

            for segment in 0..=self.resolution {
                let theta = segment as f32 * step_theta;
                let (sin, cos) = theta.sin_cos();

                positions.push([radius * cos, y, radius * sin]);
                normals.push(
                    Vec3::new(cos, (radius_bottom - radius_top) / height, sin)
                        .normalize()
                        .to_array(),
                );
                uvs.push([
                    segment as f32 / self.resolution as f32,
                    ring as f32 / self.segments as f32,
                ]);
            }
        }

        // barrel skin

        for i in 0..self.segments {
            let ring = i * (self.resolution + 1);
            let next_ring = (i + 1) * (self.resolution + 1);

            for j in 0..self.resolution {
                indices.extend_from_slice(&[
                    ring + j,
                    next_ring + j,
                    ring + j + 1,
                    next_ring + j,
                    next_ring + j + 1,
                    ring + j + 1,
                ]);
            }
        }

        // caps

        let mut build_cap = |top: bool| {
            let offset = positions.len() as u32;
            let (y, normal_y, winding) = if top {
                (height / 2., 1., (1, 0))
            } else {
                (height / -2., -1., (0, 1))
            };

            let radius = if top { radius_top } else { radius_bottom };

            if radius == 0.0 {
                return;
            }

            for i in 0..self.resolution {
                let theta = i as f32 * step_theta;
                let (sin, cos) = theta.sin_cos();

                positions.push([cos * radius, y, sin * radius]);
                normals.push([0.0, normal_y, 0.0]);
                uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);
            }

            for i in 1..(self.resolution - 1) {
                indices.extend_from_slice(&[
                    offset,
                    offset + i + winding.0,
                    offset + i + winding.1,
                ]);
            }
        };

        // top

        build_cap(true);
        build_cap(false);

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_indices(Some(Indices::U32(indices)))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for ConicalFrustum {
    type Output = ConicalFrustumBuilder;

    fn mesh(&self) -> Self::Output {
        ConicalFrustumBuilder {
            frustum: *self,
            resolution: 32,
            segments: 1,
        }
    }
}

impl From<ConicalFrustum> for Mesh {
    fn from(frustum: ConicalFrustum) -> Self {
        frustum.mesh().build()
    }
}

impl From<ConicalFrustumBuilder> for Mesh {
    fn from(frustum: ConicalFrustumBuilder) -> Self {
        frustum.build()
    }
}
