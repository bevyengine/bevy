use bevy_math::{
    primitives::{Annulus, Capsule2d, Circle, Ellipse, Extrusion, Primitive2d},
    Vec2, Vec3,
};

use crate::mesh::{Indices, Mesh, VertexAttributeValues};

use super::{MeshBuilder, Meshable};

/// A type representing a segment of the perimeter of an extrudable mesh.
pub enum PerimeterSegment {
    /// This segment of the perimeter will be shaded smooth.
    ///
    /// This has the effect of rendering the segment's faces with softened edges, so it is appropriate for curved shapes.
    Smooth {
        /// The normal of the first vertex.
        first_normal: Vec2,
        /// The normal of the last vertex.
        last_normal: Vec2,
        /// A list of indices representing this segment of the perimeter of the mesh.
        ///
        /// The indices must be ordered such that the *outside* of the mesh is to the right
        /// when walking along the vertices of the mesh in the order provided by the indices.
        ///
        /// For geometry to be rendered, you must provide at least two indices.
        indices: Vec<u32>,
    },
    /// This segment of the perimeter will be shaded flat.
    ///
    /// This has the effect of rendering the segment's faces with hard edges.
    Flat {
        /// A list of indices representing this segment of the perimeter of the mesh.
        ///
        /// The indices must be ordered such that the *outside* of the mesh is to the right
        /// when walking along the vertices of the mesh in the order provided by indices.
        ///
        /// For geometry to be rendered, you must provide at least two indices.
        indices: Vec<u32>,
    },
}

impl PerimeterSegment {
    fn vertices_per_layer(&self) -> usize {
        match self {
            PerimeterSegment::Smooth { indices, .. } => indices.len(),
            PerimeterSegment::Flat { indices } => 2 * (indices.len() - 1),
        }
    }
    fn indices_per_segment(&self) -> usize {
        match self {
            PerimeterSegment::Smooth { indices, .. } | PerimeterSegment::Flat { indices } => {
                6 * (indices.len() - 1)
            }
        }
    }
}

/// A trait for required for implementing `Meshable` for `Extrusion<T>`.
///
/// ## Warning
///
/// By implementing this trait you guarantee that the `primitive_topology` of the mesh returned by this builder is [`PrimitiveTopology::TriangleList`](wgpu::PrimitiveTopology::TriangleList)
/// and that your mesh has a [`Mesh::ATTRIBUTE_POSITION`] attribute.
pub trait Extrudable: MeshBuilder {
    /// A list of the indices each representing a part of the perimeter of the mesh.
    fn perimeter(&self) -> Vec<PerimeterSegment>;
}

impl<P> Meshable for Extrusion<P>
where
    P: Primitive2d + Meshable,
    P::Output: Extrudable,
{
    type Output = ExtrusionBuilder<P>;

    fn mesh(&self) -> Self::Output {
        ExtrusionBuilder {
            base_builder: self.base_shape.mesh(),
            half_depth: self.half_depth,
        }
    }
}

/// A builder used for creating a [`Mesh`] with an [`Extrusion`] shape.
pub struct ExtrusionBuilder<P>
where
    P: Primitive2d + Meshable,
    P::Output: Extrudable,
{
    base_builder: P::Output,
    half_depth: f32,
}

impl<P> ExtrusionBuilder<P>
where
    P: Primitive2d + Meshable,
    P::Output: Extrudable,
{
    /// Create a new `ExtrusionBuilder<P>` from a given `base_shape` and the full `depth` of the extrusion.
    pub fn new(base_shape: &P, depth: f32) -> Self {
        Self {
            base_builder: base_shape.mesh(),
            half_depth: depth / 2.,
        }
    }
}

impl ExtrusionBuilder<Circle> {
    /// Sets the number of vertices used for the circle mesh at each end of the extrusion.
    pub fn resolution(mut self, resolution: usize) -> Self {
        self.base_builder.resolution = resolution;
        self
    }
}

impl ExtrusionBuilder<Ellipse> {
    /// Sets the number of vertices used for the ellipse mesh at each end of the extrusion.
    pub fn resolution(mut self, resolution: usize) -> Self {
        self.base_builder.resolution = resolution;
        self
    }
}

impl ExtrusionBuilder<Annulus> {
    /// Sets the number of vertices used in constructing the concentric circles of the annulus mesh at each end of the extrusion.
    pub fn resolution(mut self, resolution: usize) -> Self {
        self.base_builder.resolution = resolution;
        self
    }
}

impl ExtrusionBuilder<Capsule2d> {
    /// Sets the number of vertices used for each hemicircle at the ends of the extrusion.
    pub fn resolution(mut self, resolution: usize) -> Self {
        self.base_builder.resolution = resolution;
        self
    }
}

impl<P> MeshBuilder for ExtrusionBuilder<P>
where
    P: Primitive2d + Meshable,
    P::Output: Extrudable,
{
    fn build(&self) -> Mesh {
        // Create and move the base mesh to the front
        let mut front_face =
            self.base_builder
                .build()
                .translated_by(Vec3::new(0., 0., self.half_depth));

        // Move the uvs of the front face to be between (0., 0.) and (0.5, 0.5)
        if let Some(VertexAttributeValues::Float32x2(uvs)) =
            front_face.attribute_mut(Mesh::ATTRIBUTE_UV_0)
        {
            for uv in uvs {
                *uv = uv.map(|coord| coord * 0.5);
            }
        }

        let back_face = {
            let topology = front_face.primitive_topology();
            // Flip the normals, etc. and move mesh to the back
            let mut back_face = front_face.clone().scaled_by(Vec3::new(1., 1., -1.));

            // Move the uvs of the back face to be between (0.5, 0.) and (1., 0.5)
            if let Some(VertexAttributeValues::Float32x2(uvs)) =
                back_face.attribute_mut(Mesh::ATTRIBUTE_UV_0)
            {
                for uv in uvs {
                    *uv = [uv[0] + 0.5, uv[1]];
                }
            }

            // By swapping the first and second indices of each triangle we invert the winding order thus making the mesh visible from the other side
            if let Some(indices) = back_face.indices_mut() {
                match topology {
                    wgpu::PrimitiveTopology::TriangleList => match indices {
                        Indices::U16(indices) => {
                            indices.chunks_exact_mut(3).for_each(|arr| arr.swap(1, 0));
                        }
                        Indices::U32(indices) => {
                            indices.chunks_exact_mut(3).for_each(|arr| arr.swap(1, 0));
                        }
                    },
                    _ => {
                        panic!("Meshes used with Extrusions must have a primitive topology of `PrimitiveTopology::TriangleList`");
                    }
                };
            }
            back_face
        };

        // An extrusion of depth 0 does not need a mantel
        if self.half_depth == 0. {
            front_face.merge(back_face);
            return front_face;
        }

        let mantel = {
            let Some(VertexAttributeValues::Float32x3(cap_verts)) =
                front_face.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("The base mesh did not have vertex positions");
            };

            let perimeter = self.base_builder.perimeter();
            let (vert_count, index_count) =
                perimeter
                    .iter()
                    .fold((0, 0), |(verts, indices), perimeter| {
                        (
                            verts + 2 * perimeter.vertices_per_layer(),
                            indices + perimeter.indices_per_segment(),
                        )
                    });
            let mut positions = Vec::with_capacity(vert_count);
            let mut normals = Vec::with_capacity(vert_count);
            let mut indices = Vec::with_capacity(index_count);
            let mut uvs = Vec::with_capacity(vert_count);

            let uv_segment_delta = 1. / perimeter.len() as f32;
            for (i, skin) in perimeter.into_iter().enumerate() {
                let uv_start = i as f32 * uv_segment_delta;

                match skin {
                    PerimeterSegment::Flat {
                        indices: skin_indices,
                    } => {
                        let uv_delta = uv_segment_delta / (skin_indices.len() - 1) as f32;
                        for i in 0..(skin_indices.len() - 1) {
                            let uv_x = uv_start + uv_delta * i as f32;
                            let a = cap_verts[skin_indices[i] as usize];
                            let b = cap_verts[skin_indices[i + 1] as usize];
                            let index = positions.len() as u32;

                            positions.push(a);
                            positions.push(b);
                            positions.push([a[0], a[1], -a[2]]);
                            positions.push([b[0], b[1], -b[2]]);

                            uvs.extend_from_slice(&[
                                [uv_x, 0.5],
                                [uv_x + uv_delta, 0.5],
                                [uv_x, 1.],
                                [uv_x + uv_delta, 1.],
                            ]);

                            let n = Vec3::from_array([b[1] - a[1], a[0] - b[0], 0.])
                                .normalize_or_zero()
                                .to_array();
                            normals.extend_from_slice(&[n; 4]);

                            indices.extend_from_slice(&[
                                index,
                                index + 2,
                                index + 1,
                                index + 1,
                                index + 2,
                                index + 3,
                            ]);
                        }
                    }
                    PerimeterSegment::Smooth {
                        first_normal,
                        last_normal,
                        indices: skin_indices,
                    } => {
                        let uv_delta = uv_segment_delta / (skin_indices.len() - 1) as f32;
                        let base_index = positions.len() as u32;

                        if let Some(i) = skin_indices.first() {
                            let p = cap_verts[*i as usize];
                            positions.push(p);
                            positions.push([p[0], p[1], -p[2]]);
                            uvs.extend_from_slice(&[[uv_start, 0.5], [uv_start, 1.]]);
                            normals.extend_from_slice(&[first_normal.extend(0.).to_array(); 2]);
                        }
                        for i in 1..(skin_indices.len() - 1) {
                            let uv_x = uv_start + uv_delta * i as f32;
                            let a = cap_verts[skin_indices[i - 1] as usize];
                            let b = cap_verts[skin_indices[i] as usize];
                            let c = cap_verts[skin_indices[i + 1] as usize];

                            positions.push(b);
                            positions.push([b[0], b[1], -b[2]]);

                            uvs.extend_from_slice(&[[uv_x, 0.5], [uv_x, 1.]]);

                            let n = {
                                let ab = Vec2::from_slice(&b) - Vec2::from_slice(&a);
                                let bc = Vec2::from_slice(&c) - Vec2::from_slice(&b);
                                let n = ab + bc;
                                Vec2::new(n.y, -n.x).normalize().extend(0.).to_array()
                            };
                            normals.extend_from_slice(&[n; 2]);
                        }
                        if let Some(i) = skin_indices.last() {
                            let p = cap_verts[*i as usize];
                            positions.push(p);
                            positions.push([p[0], p[1], -p[2]]);
                            uvs.extend_from_slice(&[
                                [uv_start + uv_segment_delta, 0.5],
                                [uv_start + uv_segment_delta, 1.],
                            ]);
                            normals.extend_from_slice(&[last_normal.extend(0.).to_array(); 2]);
                        }

                        for i in 0..(skin_indices.len() as u32 - 1) {
                            let index = base_index + 2 * i;
                            indices.extend_from_slice(&[
                                index,
                                index + 1,
                                index + 2,
                                index + 2,
                                index + 1,
                                index + 3,
                            ]);
                        }
                    }
                }
            }

            Mesh::new(
                wgpu::PrimitiveTopology::TriangleList,
                front_face.asset_usage,
            )
            .with_inserted_indices(Indices::U32(indices))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        };

        front_face.merge(back_face);
        front_face.merge(mantel);
        front_face
    }
}

impl<P> From<Extrusion<P>> for Mesh
where
    P: Primitive2d + Meshable,
    P::Output: Extrudable,
{
    fn from(value: Extrusion<P>) -> Self {
        value.mesh().build()
    }
}
