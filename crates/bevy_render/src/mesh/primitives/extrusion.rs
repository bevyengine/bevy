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
    ///
    /// The normals for the vertices that are part of this segment will be calculated based on the positions of their neighbours.
    /// Each normal is interpolated between the normals of the two line segments connecting it with its neighbours.
    /// Closer vertices have a stronger effect on the normal than more distant ones.
    ///
    /// Since the vertices corresponding to the first and last indices do not have two neighbouring vertices, their normals must be provided manually.
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
    /// Returns the amount of vertices each 'layer' of the extrusion should include for this perimeter segment.
    ///
    /// A layer is the set of vertices sharing a common Z value or depth.
    fn vertices_per_layer(&self) -> u32 {
        match self {
            PerimeterSegment::Smooth { indices, .. } => indices.len() as u32,
            PerimeterSegment::Flat { indices } => 2 * (indices.len() as u32 - 1),
        }
    }

    /// Returns the amount of indices each 'segment' of the extrusion should include for this perimeter segment.
    ///
    /// A segment is the set of faces on the mantel of the extrusion between two layers of vertices.
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
/// By implementing this trait you guarantee that the `primitive_topology` of the mesh returned by
/// this builder is [`PrimitiveTopology::TriangleList`](wgpu::PrimitiveTopology::TriangleList)
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
            segments: 1,
        }
    }
}

/// A builder used for creating a [`Mesh`] with an [`Extrusion`] shape.
pub struct ExtrusionBuilder<P>
where
    P: Primitive2d + Meshable,
    P::Output: Extrudable,
{
    pub base_builder: P::Output,
    pub half_depth: f32,
    pub segments: usize,
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
            segments: 1,
        }
    }

    /// Sets the number of segments along the depth of the extrusion.
    /// Must be greater than `0` for the geometry of the mantel to be generated.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl ExtrusionBuilder<Circle> {
    /// Sets the number of vertices used for the circle mesh at each end of the extrusion.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.base_builder.resolution = resolution;
        self
    }
}

impl ExtrusionBuilder<Ellipse> {
    /// Sets the number of vertices used for the ellipse mesh at each end of the extrusion.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.base_builder.resolution = resolution;
        self
    }
}

impl ExtrusionBuilder<Annulus> {
    /// Sets the number of vertices used in constructing the concentric circles of the annulus mesh at each end of the extrusion.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.base_builder.resolution = resolution;
        self
    }
}

impl ExtrusionBuilder<Capsule2d> {
    /// Sets the number of vertices used for each hemicircle at the ends of the extrusion.
    pub fn resolution(mut self, resolution: u32) -> Self {
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
            front_face.merge(&back_face);
            return front_face;
        }

        let mantel = {
            let Some(VertexAttributeValues::Float32x3(cap_verts)) =
                front_face.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("The base mesh did not have vertex positions");
            };

            debug_assert!(self.segments > 0);

            let layers = self.segments + 1;
            let layer_depth_delta = self.half_depth * 2.0 / self.segments as f32;

            let perimeter = self.base_builder.perimeter();
            let (vert_count, index_count) =
                perimeter
                    .iter()
                    .fold((0, 0), |(verts, indices), perimeter| {
                        (
                            verts + layers * perimeter.vertices_per_layer() as usize,
                            indices + self.segments * perimeter.indices_per_segment(),
                        )
                    });
            let mut positions = Vec::with_capacity(vert_count);
            let mut normals = Vec::with_capacity(vert_count);
            let mut indices = Vec::with_capacity(index_count);
            let mut uvs = Vec::with_capacity(vert_count);

            // Compute the amount of horizontal space allocated to each segment of the perimeter.
            let uv_segment_delta = 1. / perimeter.len() as f32;
            for (i, segment) in perimeter.into_iter().enumerate() {
                // The start of the x range of the area of the current perimeter-segment.
                let uv_start = i as f32 * uv_segment_delta;

                match segment {
                    PerimeterSegment::Flat {
                        indices: segment_indices,
                    } => {
                        let uv_delta = uv_segment_delta / (segment_indices.len() - 1) as f32;
                        for i in 0..(segment_indices.len() - 1) {
                            let uv_x = uv_start + uv_delta * i as f32;
                            // Get the positions for the current and the next index.
                            let a = cap_verts[segment_indices[i] as usize];
                            let b = cap_verts[segment_indices[i + 1] as usize];

                            // Get the index of the next vertex added to the mantel.
                            let index = positions.len() as u32;

                            // Push the positions of the two indices and their equivalent points on each layer.
                            for i in 0..layers {
                                let i = i as f32;
                                let z = a[2] - layer_depth_delta * i;
                                positions.push([a[0], a[1], z]);
                                positions.push([b[0], b[1], z]);

                                // UVs for the mantel are between (0, 0.5) and (1, 1).
                                let uv_y = 0.5 + 0.5 * i / self.segments as f32;
                                uvs.push([uv_x, uv_y]);
                                uvs.push([uv_x + uv_delta, uv_y]);
                            }

                            // The normal is calculated to be the normal of the line segment connecting a and b.
                            let n = Vec3::from_array([b[1] - a[1], a[0] - b[0], 0.])
                                .normalize_or_zero()
                                .to_array();
                            normals.extend_from_slice(&vec![n; 2 * layers]);

                            // Add the indices for the vertices created above to the mesh.
                            for i in 0..self.segments as u32 {
                                let base_index = index + 2 * i;
                                indices.extend_from_slice(&[
                                    base_index,
                                    base_index + 2,
                                    base_index + 1,
                                    base_index + 1,
                                    base_index + 2,
                                    base_index + 3,
                                ]);
                            }
                        }
                    }
                    PerimeterSegment::Smooth {
                        first_normal,
                        last_normal,
                        indices: segment_indices,
                    } => {
                        let uv_delta = uv_segment_delta / (segment_indices.len() - 1) as f32;

                        // Since the indices for this segment will be added after its vertices have been added,
                        // we need to store the index of the first vertex that is part of this segment.
                        let base_index = positions.len() as u32;

                        // If there is a first vertex, we need to add it and its counterparts on each layer.
                        // The normal is provided by `segment.first_normal`.
                        if let Some(i) = segment_indices.first() {
                            let p = cap_verts[*i as usize];
                            for i in 0..layers {
                                let i = i as f32;
                                let z = p[2] - layer_depth_delta * i;
                                positions.push([p[0], p[1], z]);

                                let uv_y = 0.5 + 0.5 * i / self.segments as f32;
                                uvs.push([uv_start, uv_y]);
                            }
                            normals.extend_from_slice(&vec![
                                first_normal.extend(0.).to_array();
                                layers
                            ]);
                        }

                        // For all points inbetween the first and last vertices, we can automatically compute the normals.
                        for i in 1..(segment_indices.len() - 1) {
                            let uv_x = uv_start + uv_delta * i as f32;

                            // Get the positions for the last, current and the next index.
                            let a = cap_verts[segment_indices[i - 1] as usize];
                            let b = cap_verts[segment_indices[i] as usize];
                            let c = cap_verts[segment_indices[i + 1] as usize];

                            // Add the current vertex and its counterparts on each layer.
                            for i in 0..layers {
                                let i = i as f32;
                                let z = b[2] - layer_depth_delta * i;
                                positions.push([b[0], b[1], z]);

                                let uv_y = 0.5 + 0.5 * i / self.segments as f32;
                                uvs.push([uv_x, uv_y]);
                            }

                            // The normal for the current vertices can be calculated based on the two neighbouring vertices.
                            // The normal is interpolated between the normals of the two line segments connecting the current vertex with its neighbours.
                            // Closer vertices have a stronger effect on the normal than more distant ones.
                            let n = {
                                let ab = Vec2::from_slice(&b) - Vec2::from_slice(&a);
                                let bc = Vec2::from_slice(&c) - Vec2::from_slice(&b);
                                let n = ab.normalize_or_zero() + bc.normalize_or_zero();
                                Vec2::new(n.y, -n.x)
                                    .normalize_or_zero()
                                    .extend(0.)
                                    .to_array()
                            };
                            normals.extend_from_slice(&vec![n; layers]);
                        }

                        // If there is a last vertex, we need to add it and its counterparts on each layer.
                        // The normal is provided by `segment.last_normal`.
                        if let Some(i) = segment_indices.last() {
                            let p = cap_verts[*i as usize];
                            for i in 0..layers {
                                let i = i as f32;
                                let z = p[2] - layer_depth_delta * i;
                                positions.push([p[0], p[1], z]);

                                let uv_y = 0.5 + 0.5 * i / self.segments as f32;
                                uvs.push([uv_start + uv_segment_delta, uv_y]);
                            }
                            normals.extend_from_slice(&vec![
                                last_normal.extend(0.).to_array();
                                layers
                            ]);
                        }

                        let columns = segment_indices.len() as u32;
                        let segments = self.segments as u32;
                        let layers = segments + 1;
                        for s in 0..segments {
                            for column in 0..(columns - 1) {
                                let index = base_index + s + column * layers;
                                indices.extend_from_slice(&[
                                    index,
                                    index + 1,
                                    index + layers,
                                    index + layers,
                                    index + 1,
                                    index + layers + 1,
                                ]);
                            }
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

        front_face.merge(&back_face);
        front_face.merge(&mantel);
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
