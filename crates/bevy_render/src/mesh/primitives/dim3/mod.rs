use bevy_math::{
    primitives::{Cuboid, Cylinder, Direction3d, Plane3d, Torus},
    Quat, Vec2, Vec3,
};
use wgpu::PrimitiveTopology;

use crate::{
    mesh::{Indices, Mesh},
    render_asset::RenderAssetUsages,
};

use super::Meshable;

mod capsule;
mod sphere;

pub use capsule::*;
pub use sphere::*;

impl Meshable for Cuboid {
    type Output = Mesh;

    fn mesh(&self) -> Self::Output {
        let min = -self.half_size;
        let max = self.half_size;

        // Suppose Y-up right hand, and camera look from +Z to -Z
        let vertices = &[
            // Front
            ([min.x, min.y, max.z], [0.0, 0.0, 1.0], [0.0, 0.0]),
            ([max.x, min.y, max.z], [0.0, 0.0, 1.0], [1.0, 0.0]),
            ([max.x, max.y, max.z], [0.0, 0.0, 1.0], [1.0, 1.0]),
            ([min.x, max.y, max.z], [0.0, 0.0, 1.0], [0.0, 1.0]),
            // Back
            ([min.x, max.y, min.z], [0.0, 0.0, -1.0], [1.0, 0.0]),
            ([max.x, max.y, min.z], [0.0, 0.0, -1.0], [0.0, 0.0]),
            ([max.x, min.y, min.z], [0.0, 0.0, -1.0], [0.0, 1.0]),
            ([min.x, min.y, min.z], [0.0, 0.0, -1.0], [1.0, 1.0]),
            // Right
            ([max.x, min.y, min.z], [1.0, 0.0, 0.0], [0.0, 0.0]),
            ([max.x, max.y, min.z], [1.0, 0.0, 0.0], [1.0, 0.0]),
            ([max.x, max.y, max.z], [1.0, 0.0, 0.0], [1.0, 1.0]),
            ([max.x, min.y, max.z], [1.0, 0.0, 0.0], [0.0, 1.0]),
            // Left
            ([min.x, min.y, max.z], [-1.0, 0.0, 0.0], [1.0, 0.0]),
            ([min.x, max.y, max.z], [-1.0, 0.0, 0.0], [0.0, 0.0]),
            ([min.x, max.y, min.z], [-1.0, 0.0, 0.0], [0.0, 1.0]),
            ([min.x, min.y, min.z], [-1.0, 0.0, 0.0], [1.0, 1.0]),
            // Top
            ([max.x, max.y, min.z], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ([min.x, max.y, min.z], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ([min.x, max.y, max.z], [0.0, 1.0, 0.0], [0.0, 1.0]),
            ([max.x, max.y, max.z], [0.0, 1.0, 0.0], [1.0, 1.0]),
            // Bottom
            ([max.x, min.y, max.z], [0.0, -1.0, 0.0], [0.0, 0.0]),
            ([min.x, min.y, max.z], [0.0, -1.0, 0.0], [1.0, 0.0]),
            ([min.x, min.y, min.z], [0.0, -1.0, 0.0], [1.0, 1.0]),
            ([max.x, min.y, min.z], [0.0, -1.0, 0.0], [0.0, 1.0]),
        ];

        let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
        let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
        let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

        let indices = Indices::U32(vec![
            0, 1, 2, 2, 3, 0, // front
            4, 5, 6, 6, 7, 4, // back
            8, 9, 10, 10, 11, 8, // right
            12, 13, 14, 14, 15, 12, // left
            16, 17, 18, 18, 19, 16, // top
            20, 21, 22, 22, 23, 20, // bottom
        ]);

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_indices(Some(indices))
    }
}

impl From<Cuboid> for Mesh {
    fn from(cuboid: Cuboid) -> Self {
        cuboid.mesh()
    }
}

/// A builder used for creating a [`Mesh`] with a [`Plane3d`] shape.
#[derive(Clone, Copy, Debug)]
pub struct PlaneMeshBuilder {
    /// The [`Plane3d`] shape.
    pub plane: Plane3d,
    /// Half the size of the plane mesh.
    pub half_size: Vec2,
}

impl Default for PlaneMeshBuilder {
    fn default() -> Self {
        Self {
            plane: Plane3d::default(),
            half_size: Vec2::ONE,
        }
    }
}

impl PlaneMeshBuilder {
    /// Creates a new [`PlaneMeshBuilder`] from a given normal and size.
    ///
    /// # Panics
    ///
    /// Panics if the given `normal` is zero (or very close to zero), or non-finite.
    #[inline]
    pub fn new(normal: Direction3d, size: Vec2) -> Self {
        Self {
            plane: Plane3d { normal },
            half_size: size / 2.0,
        }
    }

    /// Creates a new [`PlaneMeshBuilder`] from the given size, with the normal pointing upwards.
    #[inline]
    pub fn from_size(size: Vec2) -> Self {
        Self {
            half_size: size / 2.0,
            ..Default::default()
        }
    }

    /// Sets the normal of the plane, aka the direction the plane is facing.
    ///
    /// # Panics
    ///
    /// Panics if the given `normal` is zero (or very close to zero), or non-finite.
    #[inline]
    #[doc(alias = "facing")]
    pub fn normal(mut self, normal: Direction3d) -> Self {
        self.plane = Plane3d { normal };
        self
    }

    /// Sets the size of the plane mesh.
    #[inline]
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.half_size = Vec2::new(width, height) / 2.0;
        self
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
        let rotation = Quat::from_rotation_arc(Vec3::Y, *self.plane.normal);
        let positions = vec![
            rotation * Vec3::new(self.half_size.x, 0.0, -self.half_size.y),
            rotation * Vec3::new(-self.half_size.x, 0.0, -self.half_size.y),
            rotation * Vec3::new(-self.half_size.x, 0.0, self.half_size.y),
            rotation * Vec3::new(self.half_size.x, 0.0, self.half_size.y),
        ];

        let normals = vec![self.plane.normal.to_array(); 4];
        let uvs = vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_indices(Some(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for Plane3d {
    type Output = PlaneMeshBuilder;

    fn mesh(&self) -> Self::Output {
        PlaneMeshBuilder {
            plane: *self,
            ..Default::default()
        }
    }
}

impl From<Plane3d> for Mesh {
    fn from(plane: Plane3d) -> Self {
        plane.mesh().build()
    }
}

impl From<PlaneMeshBuilder> for Mesh {
    fn from(plane: PlaneMeshBuilder) -> Self {
        plane.build()
    }
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
}

impl Default for CylinderMeshBuilder {
    fn default() -> Self {
        Self {
            cylinder: Cylinder::default(),
            resolution: 32,
            segments: 1,
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

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
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

        // top

        build_cap(true);
        build_cap(false);

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_indices(Some(Indices::U32(indices)))
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

impl From<CylinderMeshBuilder> for Mesh {
    fn from(cylinder: CylinderMeshBuilder) -> Self {
        cylinder.build()
    }
}

/// A builder used for creating a [`Mesh`] with a [`Torus`] shape.
#[derive(Clone, Copy, Debug)]
pub struct TorusMeshBuilder {
    /// The [`Torus`] shape.
    pub torus: Torus,
    /// The number of vertices used for each circular segment
    /// in the ring or tube of the torus.
    ///
    /// The default is `24`.
    pub minor_resolution: usize,
    /// The number of segments used for the main ring of the torus.
    ///
    /// A resolution of `4` would make the torus appear rectangular,
    /// while a resolution of `32` resembles a circular ring.
    ///
    /// The default is `32`.
    pub major_resolution: usize,
}

impl Default for TorusMeshBuilder {
    fn default() -> Self {
        Self {
            torus: Torus::default(),
            minor_resolution: 24,
            major_resolution: 32,
        }
    }
}

impl TorusMeshBuilder {
    /// Creates a new [`TorusMeshBuilder`] from an inner and outer radius.
    ///
    /// The inner radius is the radius of the hole, and the outer radius
    /// is the radius of the entire object.
    #[inline]
    pub fn new(inner_radius: f32, outer_radius: f32) -> Self {
        Self {
            torus: Torus::new(inner_radius, outer_radius),
            ..Default::default()
        }
    }

    /// Sets the number of vertices used for each circular segment
    /// in the ring or tube of the torus.
    #[inline]
    pub const fn minor_resolution(mut self, resolution: usize) -> Self {
        self.minor_resolution = resolution;
        self
    }

    /// Sets the number of segments used for the main ring of the torus.
    ///
    /// A resolution of `4` would make the torus appear rectangular,
    /// while a resolution of `32` resembles a circular ring.
    #[inline]
    pub const fn major_resolution(mut self, resolution: usize) -> Self {
        self.major_resolution = resolution;
        self
    }

    /// Builds a [`Mesh`] according to the configuration in `self`.
    pub fn build(&self) -> Mesh {
        // code adapted from http://apparat-engine.blogspot.com/2013/04/procedural-meshes-torus.html
        // (source code at https://github.com/SEilers/Apparat)

        let n_vertices = (self.major_resolution + 1) * (self.minor_resolution + 1);
        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(n_vertices);

        let segment_stride = 2.0 * std::f32::consts::PI / self.major_resolution as f32;
        let side_stride = 2.0 * std::f32::consts::PI / self.minor_resolution as f32;

        for segment in 0..=self.major_resolution {
            let theta = segment_stride * segment as f32;

            for side in 0..=self.minor_resolution {
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
                    segment as f32 / self.major_resolution as f32,
                    side as f32 / self.minor_resolution as f32,
                ]);
            }
        }

        let n_faces = (self.major_resolution) * (self.minor_resolution);
        let n_triangles = n_faces * 2;
        let n_indices = n_triangles * 3;

        let mut indices: Vec<u32> = Vec::with_capacity(n_indices);

        let n_vertices_per_row = self.minor_resolution + 1;
        for segment in 0..self.major_resolution {
            for side in 0..self.minor_resolution {
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
            RenderAssetUsages::default(),
        )
        .with_indices(Some(Indices::U32(indices)))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for Torus {
    type Output = TorusMeshBuilder;

    fn mesh(&self) -> Self::Output {
        TorusMeshBuilder {
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

impl From<TorusMeshBuilder> for Mesh {
    fn from(torus: TorusMeshBuilder) -> Self {
        torus.build()
    }
}
