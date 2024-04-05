use crate::mesh::primitives::circle_iterator::CircleIterator;
use crate::{
    mesh::{Indices, Mesh, Meshable},
    render_asset::RenderAssetUsages,
};
use bevy_math::{primitives::Capsule3d, Vec2};
use wgpu::PrimitiveTopology;

/// Manner in which UV coordinates are distributed vertically.
#[derive(Clone, Copy, Debug, Default)]
pub enum CapsuleUvProfile {
    /// UV space is distributed by how much of the capsule consists of the hemispheres.
    #[default]
    Aspect,
    /// Hemispheres get UV space according to the ratio of latitudes to rings.
    Uniform,
    /// Upper third of the texture goes to the northern hemisphere, middle third to the cylinder
    /// and lower third to the southern one.
    Fixed,
}

/// A builder used for creating a [`Mesh`] with a [`Capsule3d`] shape.
#[derive(Clone, Copy, Debug)]
pub struct Capsule3dMeshBuilder {
    /// The [`Capsule3d`] shape.
    pub capsule: Capsule3d,
    /// The number of horizontal lines subdividing the cylindrical part of the capsule.
    /// The default is `0`.
    pub rings: usize,
    /// The number of vertical lines subdividing the hemispheres of the capsule.
    /// The default is `32`.
    pub sectors: usize,
    /// The number of horizontal lines subdividing the hemispheres of the capsule.
    /// The default is `16`.
    pub stacks: usize,
    /// The manner in which UV coordinates are distributed vertically.
    /// The default is [`CapsuleUvProfile::Aspect`].
    pub uv_profile: CapsuleUvProfile,
}

impl Default for Capsule3dMeshBuilder {
    fn default() -> Self {
        Self {
            capsule: Capsule3d::default(),
            rings: 0,
            sectors: 32,
            stacks: 8,
            uv_profile: CapsuleUvProfile::default(),
        }
    }
}

impl Capsule3dMeshBuilder {
    /// Creates a new [`Capsule3dMeshBuilder`] from a given radius, height, longitudes, and latitudes.
    ///
    /// Note that `height` is the distance between the centers of the hemispheres.
    /// `radius` will be added to both ends to get the real height of the mesh.
    #[inline]
    pub fn new(radius: f32, height: f32, sectors: usize, stacks: usize) -> Self {
        Self {
            capsule: Capsule3d::new(radius, height),
            sectors,
            stacks,
            ..Default::default()
        }
    }

    /// Sets the number of horizontal lines subdividing the cylindrical part of the capsule.
    #[inline]
    pub const fn rings(mut self, rings: usize) -> Self {
        self.rings = rings;
        self
    }

    /// Sets the number of quad columns subdividing the hemisphere.
    #[inline]
    pub const fn sectors(mut self, sectors: usize) -> Self {
        self.sectors = sectors;
        self
    }

    /// Sets the number of quad rows along the longitude of the hemisphere.
    #[inline]
    pub const fn stacks(mut self, stacks: usize) -> Self {
        self.stacks = stacks;
        self
    }

    /// Sets the manner in which UV coordinates are distributed vertically.
    #[inline]
    pub const fn uv_profile(mut self, uv_profile: CapsuleUvProfile) -> Self {
        self.uv_profile = uv_profile;
        self
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
        // code adapted from https://behreajj.medium.com/making-a-capsule-mesh-via-script-in-five-3d-environments-c2214abf02db
        let Capsule3dMeshBuilder {
            capsule,
            rings,
            stacks,
            sectors,
            uv_profile,
        } = *self;
        let Capsule3d {
            radius,
            half_length,
        } = capsule;
        let total_stacks = 1 + rings + stacks * 2;
        let uv_aspect_ratio = match uv_profile {
            CapsuleUvProfile::Aspect => radius / (2.0 * (half_length + radius)),
            CapsuleUvProfile::Uniform => stacks as f32 / total_stacks as f32,
            CapsuleUvProfile::Fixed => 1.0 / 3.0,
        };

        // Largely inspired from http://www.songho.ca/opengl/gl_sphere.html
        let total_vertices = (2 * (stacks + 1) + rings) * (sectors + 1);
        let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(total_vertices);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(total_vertices);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(total_vertices);
        let mut indices: Vec<u32> = Vec::with_capacity(total_vertices * 2 * 3);

        let sectors_f32 = sectors as f32;
        let length_inv = 1. / radius;

        let stacks_iter = CircleIterator::quarter_circle(stacks);
        let sector_circle: Vec<Vec2> = CircleIterator::wrapping(sectors).collect();

        //insertion of top hemisphere and its associated UVs and normals
        for (i, p) in stacks_iter.enumerate() {
            let xz = radius * p.y;
            let y = radius * p.x;
            let uv_fac = i as f32 / stacks as f32;
            let v = 1.0 - uv_fac * uv_aspect_ratio;
            for (j, q) in sector_circle.iter().enumerate() {
                let x = xz * q.x;
                let z = xz * q.y;
                vertices.push([x, y + half_length, z]);
                normals.push([x * length_inv, y * length_inv, z * length_inv]);
                uvs.push([(j as f32) / sectors_f32, v]);
            }
        }

        //insertion of necessary rings along the cylinder portion of the capsule if specified
        if rings > 0 {
            let ring_circle: Vec<Vec2> = sector_circle
                .iter()
                .map(|p| Vec2 {
                    x: p.x * radius,
                    y: p.y * radius,
                })
                .collect();
            let spacing = 2.0 * half_length / (rings + 1) as f32;
            let cylinder_aspect_extent = 1.0 - 2.0 * uv_aspect_ratio;
            let cylinder_aspect_step = cylinder_aspect_extent / (rings + 1) as f32;
            for i in 1..=rings {
                for (j, point) in ring_circle.iter().enumerate() {
                    let (x, y, z) = (point.x, half_length - i as f32 * spacing, point.y);
                    vertices.push([x, y, z]);
                    normals.push([x * length_inv, 0.0, z * length_inv]);
                    uvs.push([
                        (j as f32) / sectors_f32,
                        uv_aspect_ratio + cylinder_aspect_extent - i as f32 * cylinder_aspect_step,
                    ]);
                }
            }
        }
        //insertion of bottom hemisphere and its associated UVs and normals
        //accomplished by inverting the top hemisphere
        for i in (0..=stacks).rev() {
            let uv_fac = i as f32 / stacks as f32;
            let v = uv_fac * uv_aspect_ratio;
            for j in 0..=sectors {
                let idx = i * (sectors + 1) + j;
                let [vx, vy, vz] = vertices[idx];
                let [nx, ny, nz] = normals[idx];
                vertices.push([vx, -vy, vz]);
                normals.push([nx, -ny, nz]);
                uvs.push([(j as f32) / sectors_f32, v]);
            }
        }
        // indices
        //  k1--k1+1
        //  |  / |
        //  | /  |
        //  k2--k2+1
        for i in 0..total_stacks {
            let mut k1 = i * (sectors + 1);
            let mut k2 = k1 + sectors + 1;
            for _j in 0..sectors {
                if i != 0 {
                    indices.push(k2 as u32);
                    indices.push(k1 as u32);
                    indices.push((k1 + 1) as u32);
                }
                if i != total_stacks - 1 {
                    indices.push(k2 as u32);
                    indices.push((k1 + 1) as u32);
                    indices.push((k2 + 1) as u32);
                }
                k1 += 1;
                k2 += 1;
            }
        }

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for Capsule3d {
    type Output = Capsule3dMeshBuilder;

    fn mesh(&self) -> Self::Output {
        Capsule3dMeshBuilder {
            capsule: *self,
            ..Default::default()
        }
    }
}

impl From<Capsule3d> for Mesh {
    fn from(capsule: Capsule3d) -> Self {
        capsule.mesh().build()
    }
}

impl From<Capsule3dMeshBuilder> for Mesh {
    fn from(capsule: Capsule3dMeshBuilder) -> Self {
        capsule.build()
    }
}
