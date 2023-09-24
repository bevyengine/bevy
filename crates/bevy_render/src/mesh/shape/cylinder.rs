use crate::mesh::{Indices, Mesh};
use wgpu::PrimitiveTopology;

/// A cylinder which stands on the XZ plane
#[derive(Clone, Copy, Debug)]
pub struct Cylinder {
    /// Radius in the XZ plane.
    pub radius: f32,
    /// Height of the cylinder in the Y axis.
    pub height: f32,
    /// The number of vertices around each horizontal slice of the cylinder. If you are looking at the cylinder from
    /// above, this is the number of points you will see on the circle.
    /// A higher number will make it appear more circular.
    pub resolution: u32,
    /// The number of segments between the two ends. Setting this to 1 will have triangles spanning the full
    /// height of the cylinder. Setting it to 2 will have two sets of triangles with a horizontal slice in the middle of
    /// cylinder. Greater numbers increase triangles/slices in the same way.
    pub segments: u32,
}

impl Default for Cylinder {
    fn default() -> Self {
        Self {
            radius: 0.5,
            height: 1.0,
            resolution: 16,
            segments: 1,
        }
    }
}

impl From<Cylinder> for Mesh {
    fn from(c: Cylinder) -> Self {
        debug_assert!(c.radius > 0.0);
        debug_assert!(c.height > 0.0);
        debug_assert!(c.resolution > 2);
        debug_assert!(c.segments > 0);

        let num_rings = c.segments + 1;
        let num_vertices = c.resolution * 2 + num_rings * (c.resolution + 1);
        let num_faces = c.resolution * (num_rings - 2);
        let num_indices = (2 * num_faces + 2 * (c.resolution - 1) * 2) * 3;

        let mut positions = Vec::with_capacity(num_vertices as usize);
        let mut normals = Vec::with_capacity(num_vertices as usize);
        let mut uvs = Vec::with_capacity(num_vertices as usize);
        let mut indices = Vec::with_capacity(num_indices as usize);

        let step_theta = std::f32::consts::TAU / c.resolution as f32;
        let step_y = c.height / c.segments as f32;

        // rings

        for ring in 0..num_rings {
            let y = -c.height / 2.0 + ring as f32 * step_y;

            for segment in 0..=c.resolution {
                let theta = segment as f32 * step_theta;
                let (sin, cos) = theta.sin_cos();

                positions.push([c.radius * cos, y, c.radius * sin]);
                normals.push([cos, 0., sin]);
                uvs.push([
                    segment as f32 / c.resolution as f32,
                    ring as f32 / c.segments as f32,
                ]);
            }
        }

        // barrel skin

        for i in 0..c.segments {
            let ring = i * (c.resolution + 1);
            let next_ring = (i + 1) * (c.resolution + 1);

            for j in 0..c.resolution {
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
                (c.height / 2., 1., (1, 0))
            } else {
                (c.height / -2., -1., (0, 1))
            };

            for i in 0..c.resolution {
                let theta = i as f32 * step_theta;
                let (sin, cos) = theta.sin_cos();

                positions.push([cos * c.radius, y, sin * c.radius]);
                normals.push([0.0, normal_y, 0.0]);
                uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);
            }

            for i in 1..(c.resolution - 1) {
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

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
    }
}
