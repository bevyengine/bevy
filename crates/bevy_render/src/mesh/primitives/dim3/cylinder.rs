use bevy_math::primitives::Cylinder;
use wgpu::PrimitiveTopology;

use crate::{
    mesh::{Indices, Mesh, MeshBuilder, Meshable},
    render_asset::RenderAssetUsages,
};

/// Anchoring options for [`CylinderMeshBuilder`]
#[derive(Debug, Copy, Clone, Default)]
pub enum CylinderAnchor {
    #[default]
    /// Midpoint between the top and bottom caps of the cylinder
    MidPoint,
    /// The center of the top circle cap
    Top,
    /// The center of the bottom circle cap
    Bottom,
}

/// A builder used for creating a [`Mesh`] with a [`Cylinder`] shape.
#[derive(Clone, Copy, Debug)]
pub struct CylinderMeshBuilder {
    /// The [`Cylinder`] shape.
    pub cylinder: Cylinder,
    /// The number of vertices used for the top and bottom of the cylinder.
    ///
    /// The default is `32`.
    pub resolution: u32,
    /// The number of segments along the height of the cylinder.
    /// Must be greater than `0` for geometry to be generated.
    ///
    /// The default is `1`.
    pub segments: u32,
    /// If set to `true`, the cylinder caps (flat circle faces) are built,
    /// otherwise the mesh will be a shallow tube
    pub caps: bool,
    /// The anchor point for the cylinder mesh, defaults to the midpoint between
    /// the top and bottom caps
    pub anchor: CylinderAnchor,
}

impl Default for CylinderMeshBuilder {
    fn default() -> Self {
        Self {
            cylinder: Cylinder::default(),
            resolution: 32,
            segments: 1,
            caps: true,
            anchor: CylinderAnchor::default(),
        }
    }
}

impl CylinderMeshBuilder {
    /// Creates a new [`CylinderMeshBuilder`] from the given radius, a height,
    /// and a resolution used for the top and bottom.
    #[inline]
    pub fn new(radius: f32, height: f32, resolution: u32) -> Self {
        Self {
            cylinder: Cylinder::new(radius, height),
            resolution,
            ..Default::default()
        }
    }

    /// Sets the number of vertices used for the top and bottom of the cylinder.
    #[inline]
    pub const fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// Sets the number of segments along the height of the cylinder.
    /// Must be greater than `0` for geometry to be generated.
    #[inline]
    pub const fn segments(mut self, segments: u32) -> Self {
        self.segments = segments;
        self
    }

    /// Ignore the cylinder caps, making the mesh a shallow tube instead
    #[inline]
    pub const fn without_caps(mut self) -> Self {
        self.caps = false;
        self
    }

    /// Sets a custom anchor point for the mesh
    #[inline]
    pub const fn anchor(mut self, anchor: CylinderAnchor) -> Self {
        self.anchor = anchor;
        self
    }
}

impl MeshBuilder for CylinderMeshBuilder {
    fn build(&self) -> Mesh {
        let resolution = self.resolution;
        let segments = self.segments;

        debug_assert!(resolution > 2);
        debug_assert!(segments > 0);

        let num_rings = segments + 1;
        let num_vertices = resolution * 2 + num_rings * (resolution + 1);
        let num_faces = resolution * (num_rings - 2);
        let num_indices = (2 * num_faces + 2 * (resolution - 1) * 2) * 3;

        let mut positions = Vec::with_capacity(num_vertices as usize);
        let mut normals = Vec::with_capacity(num_vertices as usize);
        let mut uvs = Vec::with_capacity(num_vertices as usize);
        let mut indices = Vec::with_capacity(num_indices as usize);

        let step_theta = std::f32::consts::TAU / resolution as f32;
        let step_y = 2.0 * self.cylinder.half_height / segments as f32;

        // rings

        for ring in 0..num_rings {
            let y = -self.cylinder.half_height + ring as f32 * step_y;

            for segment in 0..=resolution {
                let theta = segment as f32 * step_theta;
                let (sin, cos) = theta.sin_cos();

                positions.push([self.cylinder.radius * cos, y, self.cylinder.radius * sin]);
                normals.push([cos, 0., sin]);
                uvs.push([
                    segment as f32 / resolution as f32,
                    ring as f32 / segments as f32,
                ]);
            }
        }

        // barrel skin

        for i in 0..segments {
            let ring = i * (resolution + 1);
            let next_ring = (i + 1) * (resolution + 1);

            for j in 0..resolution {
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
        if self.caps {
            let mut build_cap = |top: bool| {
                let offset = positions.len() as u32;
                let (y, normal_y, winding) = if top {
                    (self.cylinder.half_height, 1., (1, 0))
                } else {
                    (-self.cylinder.half_height, -1., (0, 1))
                };

                for i in 0..self.resolution {
                    let theta = i as f32 * step_theta;
                    let (sin, cos) = theta.sin_cos();

                    positions.push([cos * self.cylinder.radius, y, sin * self.cylinder.radius]);
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

            build_cap(true);
            build_cap(false);
        }

        // Offset the vertex positions Y axis to match the anchor
        match self.anchor {
            CylinderAnchor::Top => positions
                .iter_mut()
                .for_each(|p| p[1] -= self.cylinder.half_height),
            CylinderAnchor::Bottom => positions
                .iter_mut()
                .for_each(|p| p[1] += self.cylinder.half_height),
            CylinderAnchor::MidPoint => (),
        };

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for Cylinder {
    type Output = CylinderMeshBuilder;

    fn mesh(&self) -> Self::Output {
        CylinderMeshBuilder {
            cylinder: *self,
            ..Default::default()
        }
    }
}

impl From<Cylinder> for Mesh {
    fn from(cylinder: Cylinder) -> Self {
        cylinder.mesh().build()
    }
}
