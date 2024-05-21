use bevy_math::{
    primitives::{Annulus, Capsule2d, Circle, Ellipse, Extrusion, Primitive2d},
    Vec2, Vec3,
};

use crate::mesh::{Indices, Mesh, VertexAttributeValues};

use super::{MeshBuilder, Meshable};

/// An enum representing segments of the perimeter of extrudable meshes
pub enum PerimeterSegment {
    /// This segment of the perimeter will be shaded smooth.
    ///
    /// You may want to use this for curved segments
    Smooth {
        /// The normal of the first vertex
        first_normal: Vec2,
        /// The normal of the last vertex
        last_normal: Vec2,
        /// A list of indices representing this segment of the perimeter of the mesh
        ///
        /// The indices must be ordered such that the *outside* of the mesh is to the right
        /// when walking along the vertices of the mesh in the order provided by indices
        indices: Vec<u32>,
    },
    /// This segment of the perimeter will be shaded flat.
    ///
    /// You may want to use this if there are sharp corners in the perimeter
    Flat {
        /// A list of indices representing this segment of the perimeter of the mesh
        ///
        /// The indices must be ordered such that the *outside* of the mesh is to the right
        /// when walking along the vertices of the mesh in the order provided by indices
        indices: Vec<u32>,
    },
}

/// A trait for required for implementing `Meshable` for `Extrusion<T>`
///
/// ## Warning
///
/// By implementing this trait you guarantee that the `primitive_topology` of the mesh returned by this builder is [`PrimitiveTopology::TriangleList`](wgpu::PrimitiveTopology::TriangleList)
/// and that your mesh has a [`Mesh::ATTRIBUTE_POSITION`] attribute
pub trait Extrudable: MeshBuilder {
    /// A list of the indices each representing a part of the perimeter of the mesh.
    fn perimeter_indices(&self) -> Vec<PerimeterSegment>;
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
    base_builder: P::Output,
    half_depth: f32,
}

impl<P> MeshBuilder for ExtrusionBuilder<P>
where
    P: Primitive2d + Meshable,
    P::Output: Extrudable,
{
    fn build(&self) -> Mesh {
        build_extrusion(
            self.base_builder.build(),
            self.base_builder.perimeter_indices(),
            self.half_depth,
        )
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

fn build_extrusion(cap: Mesh, perimeter: Vec<PerimeterSegment>, half_depth: f32) -> Mesh {
    // Move the base mesh to the front
    let mut front_face = cap.translated_by(Vec3::new(0., 0., half_depth));

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
                *uv = [uv[0] + 0.5, uv[1]]
            }
        }

        // By swapping the first and second indices of each triangle we invert the winding order thus making the mesh visible from the other side
        if let Some(indices) = back_face.indices_mut() {
            match topology {
                wgpu::PrimitiveTopology::TriangleList => match indices {
                    Indices::U16(indices) => {
                        indices.chunks_exact_mut(3).for_each(|arr| arr.swap(1, 0))
                    }
                    Indices::U32(indices) => {
                        indices.chunks_exact_mut(3).for_each(|arr| arr.swap(1, 0))
                    }
                },
                _ => {
                    panic!("Meshes used with Extrusions must have a primitive topology of either `PrimitiveTopology::TriangleList`");
                }
            };
        }
        back_face
    };

    // An extrusion of depth 0 does not need a mantel
    if half_depth == 0. {
        front_face.merge(back_face);
        return front_face;
    }

    let mantel = {
        let Some(VertexAttributeValues::Float32x3(cap_verts)) =
            front_face.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("The cap mesh did not have a vertex attribute");
        };

        let vert_count = perimeter
            .iter()
            .fold(0, |acc, indices| acc + indices.len() - 1);
        let mut positions = Vec::with_capacity(vert_count * 4);
        let mut normals = Vec::with_capacity(vert_count * 4);
        let mut indices = Vec::with_capacity(vert_count * 6);
        let mut uvs = Vec::with_capacity(vert_count * 4);

        let uv_delta = 1. / (vert_count + perimeter.len() - 1) as f32;
        let mut uv_x = 0.;
        let mut index = 0;
        for skin in perimeter {
            let skin_indices = match skin {
                Indices::U16(ind) => ind.into_iter().map(|i| i as u32).collect(),
                Indices::U32(ind) => ind,
            };
            for i in 0..(skin_indices.len() - 1) {
                let a = cap_verts[skin_indices[i] as usize];
                let b = cap_verts[skin_indices[i + 1] as usize];

                positions.push(a);
                positions.push(b);
                positions.push([a[0], a[1], -a[2]]);
                positions.push([b[0], b[1], -b[2]]);

                uvs.append(&mut vec![
                    [uv_x, 0.5],
                    [uv_x + uv_delta, 0.5],
                    [uv_x, 1.],
                    [uv_x + uv_delta, 1.],
                ]);

                let n = Vec3::from_array([b[1] - a[1], a[0] - b[0], 0.])
                    .normalize_or_zero()
                    .to_array();
                normals.append(&mut vec![n; 4]);

                indices.append(&mut vec![
                    index,
                    index + 2,
                    index + 1,
                    index + 1,
                    index + 2,
                    index + 3,
                ]);

                index += 4;
                uv_x += uv_delta;
            }

            uv_x += uv_delta;
        }

        Mesh::new(front_face.primitive_topology(), front_face.asset_usage)
            .with_inserted_indices(Indices::U32(indices))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    };

    front_face.merge(back_face);
    front_face.merge(mantel);
    front_face
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
