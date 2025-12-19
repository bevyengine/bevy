use crate::{
    triangle_area_normal, triangle_normal, FourIterators, Indices, Mesh, MeshAttributeData,
    MeshMergeError, MeshTrianglesError, MeshVertexAttribute, MeshVertexAttributeId,
    MeshVertexBufferLayout, MeshVertexBufferLayoutRef, MeshVertexBufferLayouts,
    MeshWindingInvertError, VertexAttributeValues, VertexBufferLayout,
};
use alloc::collections::BTreeMap;
#[cfg(feature = "morph")]
use bevy_asset::Handle;
use bevy_asset::{Asset, ExtractableAssetAccessError};
#[cfg(feature = "morph")]
use bevy_image::Image;
use bevy_math::{primitives::Triangle3d, Quat, Vec3, Vec3Swizzles as _};
use bevy_reflect::Reflect;
use bevy_transform::components::Transform;
use bytemuck::cast_slice;
use tracing::warn;
use wgpu_types::{PrimitiveTopology, VertexAttribute, VertexFormat, VertexStepMode};

#[derive(Asset, Debug, Clone, Reflect, PartialEq)]
#[reflect(Clone)]
pub struct MeshExtractableData {
    #[reflect(ignore, clone)]
    pub(crate) primitive_topology: PrimitiveTopology,
    /// `std::collections::BTreeMap` with all defined vertex attributes (Positions, Normals, ...)
    /// for this mesh. Attribute ids to attribute values.
    /// Uses a [`BTreeMap`] because, unlike `HashMap`, it has a defined iteration order,
    /// which allows easy stable `VertexBuffers` (i.e. same buffer order)
    #[reflect(ignore, clone)]
    pub(crate) attributes: BTreeMap<MeshVertexAttributeId, MeshAttributeData>,
    pub(crate) indices: Option<Indices>,
    #[cfg(feature = "morph")]
    pub(crate) morph_targets: Option<Handle<Image>>,
    #[cfg(feature = "morph")]
    pub(crate) morph_target_names: Option<Vec<String>>,
}

impl MeshExtractableData {
    /// Construct a new `MeshExtractableData`. You need to provide a [`PrimitiveTopology`] so that the
    /// renderer knows how to treat the vertex data. Most of the time this will be
    /// [`PrimitiveTopology::TriangleList`].
    pub fn new(primitive_topology: PrimitiveTopology) -> Self {
        Self {
            primitive_topology,
            attributes: Default::default(),
            indices: None,
            #[cfg(feature = "morph")]
            morph_targets: None,
            #[cfg(feature = "morph")]
            morph_target_names: None,
        }
    }

    /// Returns the topology of the `MeshExtractableData`.
    pub fn primitive_topology(&self) -> PrimitiveTopology {
        self.primitive_topology
    }

    /// Sets the data for a vertex attribute (position, normal, etc.). The name will
    /// often be one of the associated constants such as [`Mesh::ATTRIBUTE_POSITION`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    ///
    /// # Panics
    /// Panics when the format of the values does not match the attribute's format.
    #[inline]
    pub fn insert_attribute(
        &mut self,
        attribute: MeshVertexAttribute,
        values: impl Into<VertexAttributeValues>,
    ) {
        let values = values.into();
        let values_format = VertexFormat::from(&values);
        if values_format != attribute.format {
            panic!(
                "Failed to insert attribute. Invalid attribute format for {}. Given format is {values_format:?} but expected {:?}",
                attribute.name, attribute.format
            );
        }

        self.attributes
            .insert(attribute.id, MeshAttributeData { attribute, values });
    }

    /// Removes the data for a vertex attribute
    /// Returns an error if the mesh data has been extracted to `RenderWorld`or
    /// if the attribute does not exist.
    pub fn remove_attribute(
        &mut self,
        attribute: impl Into<MeshVertexAttributeId>,
    ) -> Option<VertexAttributeValues> {
        self.attributes.remove(&attribute.into()).map(|v| v.values)
    }

    /// Returns a bool indicating if the attribute is present in this mesh's vertex data.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn contains_attribute(&self, id: impl Into<MeshVertexAttributeId>) -> bool {
        self.attributes.contains_key(&id.into())
    }

    /// Retrieves the data currently set to the vertex attribute with the specified [`MeshVertexAttributeId`].
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn attribute(
        &self,
        id: impl Into<MeshVertexAttributeId>,
    ) -> Option<&VertexAttributeValues> {
        self.attributes.get(&id.into()).map(|data| &data.values)
    }

    /// Retrieves the full data currently set to the vertex attribute with the specified [`MeshVertexAttributeId`].
    #[inline]
    pub(crate) fn attribute_data(
        &self,
        id: impl Into<MeshVertexAttributeId>,
    ) -> Option<&MeshAttributeData> {
        self.attributes.get(&id.into())
    }

    /// Retrieves the data currently set to the vertex attribute with the specified `name` mutably.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn attribute_mut(
        &mut self,
        id: impl Into<MeshVertexAttributeId>,
    ) -> Option<&mut VertexAttributeValues> {
        self.attributes
            .get_mut(&id.into())
            .map(|data| &mut data.values)
    }

    /// Returns an iterator that yields references to the data of each vertex attribute.
    pub fn attributes(
        &self,
    ) -> impl Iterator<Item = (&MeshVertexAttribute, &VertexAttributeValues)> {
        self.attributes
            .values()
            .map(|data| (&data.attribute, &data.values))
    }

    /// Returns an iterator that yields mutable references to the data of each vertex attribute.
    pub fn attributes_mut(
        &mut self,
    ) -> impl Iterator<Item = (&MeshVertexAttribute, &mut VertexAttributeValues)> {
        self.attributes
            .values_mut()
            .map(|data| (&data.attribute, &mut data.values))
    }

    /// Sets the vertex indices of the mesh. They describe how triangles are constructed out of the
    /// vertex attributes and are therefore only useful for the [`PrimitiveTopology`] variants
    /// that use triangles.
    #[inline]
    pub fn insert_indices(&mut self, indices: Indices) {
        self.indices.replace(indices);
    }

    /// Retrieves the vertex `indices` of the mesh, returns None if not found.
    #[inline]
    pub fn indices(&self) -> Option<&Indices> {
        self.indices.as_ref()
    }

    /// Retrieves the vertex `indices` of the mesh mutably.
    #[inline]
    pub fn indices_mut(&mut self) -> Option<&mut Indices> {
        self.indices.as_mut()
    }

    /// Removes the vertex `indices` from the mesh and returns them.
    #[inline]
    pub fn remove_indices(&mut self) -> Option<Indices> {
        self.indices.take()
    }

    /// Returns the size of a vertex in bytes.
    pub fn get_vertex_size(&self) -> u64 {
        self.attributes
            .values()
            .map(|data| data.attribute.format.size())
            .sum()
    }

    /// Returns the size required for the vertex buffer in bytes.
    pub fn get_vertex_buffer_size(&self) -> usize {
        let vertex_size = self.get_vertex_size() as usize;
        let vertex_count = self.count_vertices();
        vertex_count * vertex_size
    }

    /// Computes and returns the index data of the mesh as bytes.
    /// This is used to transform the index data into a GPU friendly format.
    pub fn get_index_buffer_bytes(&self) -> Option<&[u8]> {
        let mesh_indices = self.indices.as_ref();

        mesh_indices.map(|indices| match &indices {
            Indices::U16(indices) => cast_slice(&indices[..]),
            Indices::U32(indices) => cast_slice(&indices[..]),
        })
    }

    /// Get this `Mesh`'s [`MeshVertexBufferLayout`], used in `SpecializedMeshPipeline`.
    pub fn get_mesh_vertex_buffer_layout(
        &self,
        mesh_vertex_buffer_layouts: &mut MeshVertexBufferLayouts,
    ) -> MeshVertexBufferLayoutRef {
        let mesh_attributes = &self.attributes;

        let mut attributes = Vec::with_capacity(mesh_attributes.len());
        let mut attribute_ids = Vec::with_capacity(mesh_attributes.len());
        let mut accumulated_offset = 0;
        for (index, data) in mesh_attributes.values().enumerate() {
            attribute_ids.push(data.attribute.id);
            attributes.push(VertexAttribute {
                offset: accumulated_offset,
                format: data.attribute.format,
                shader_location: index as u32,
            });
            accumulated_offset += data.attribute.format.size();
        }

        let layout = MeshVertexBufferLayout {
            layout: VertexBufferLayout {
                array_stride: accumulated_offset,
                step_mode: VertexStepMode::Vertex,
                attributes,
            },
            attribute_ids,
        };
        mesh_vertex_buffer_layouts.insert(layout)
    }

    /// Counts all vertices of the mesh.
    ///
    /// If the attributes have different vertex counts, the smallest is returned.
    pub fn count_vertices(&self) -> usize {
        let mut vertex_count: Option<usize> = None;
        let mesh_attributes = &self.attributes;

        for (attribute_id, attribute_data) in mesh_attributes {
            let attribute_len = attribute_data.values.len();
            if let Some(previous_vertex_count) = vertex_count {
                if previous_vertex_count != attribute_len {
                    let name = mesh_attributes
                        .get(attribute_id)
                        .map(|data| data.attribute.name.to_string())
                        .unwrap_or_else(|| format!("{attribute_id:?}"));

                    warn!("{name} has a different vertex count ({attribute_len}) than other attributes ({previous_vertex_count}) in this mesh, \
                        all attributes will be truncated to match the smallest.");
                    vertex_count = Some(core::cmp::min(previous_vertex_count, attribute_len));
                }
            } else {
                vertex_count = Some(attribute_len);
            }
        }

        vertex_count.unwrap_or(0)
    }

    /// Computes and returns the vertex data of the mesh as bytes.
    /// Therefore the attributes are located in the order of their [`MeshVertexAttribute::id`].
    /// This is used to transform the vertex data into a GPU friendly format.
    ///
    /// If the vertex attributes have different lengths, they are all truncated to
    /// the length of the smallest.
    ///
    /// This is a convenience method which allocates a Vec.
    /// Prefer pre-allocating and using [`Mesh::write_packed_vertex_buffer_data`] when possible.
    pub fn create_packed_vertex_buffer_data(&self) -> Vec<u8> {
        let mut attributes_interleaved_buffer = vec![0; self.get_vertex_buffer_size()];
        self.write_packed_vertex_buffer_data(&mut attributes_interleaved_buffer);
        attributes_interleaved_buffer
    }

    /// Computes and write the vertex data of the mesh into a mutable byte slice.
    /// The attributes are located in the order of their [`MeshVertexAttribute::id`].
    /// This is used to transform the vertex data into a GPU friendly format.
    ///
    /// If the vertex attributes have different lengths, they are all truncated to
    /// the length of the smallest.
    pub fn write_packed_vertex_buffer_data(&self, slice: &mut [u8]) {
        let mesh_attributes = &self.attributes;

        let vertex_size = self.get_vertex_size() as usize;
        let vertex_count = self.count_vertices();
        // bundle into interleaved buffers
        let mut attribute_offset = 0;
        for attribute_data in mesh_attributes.values() {
            let attribute_size = attribute_data.attribute.format.size() as usize;
            let attributes_bytes = attribute_data.values.get_bytes();
            for (vertex_index, attribute_bytes) in attributes_bytes
                .chunks_exact(attribute_size)
                .take(vertex_count)
                .enumerate()
            {
                let offset = vertex_index * vertex_size + attribute_offset;
                slice[offset..offset + attribute_size].copy_from_slice(attribute_bytes);
            }

            attribute_offset += attribute_size;
        }
    }

    /// Duplicates the vertex attributes so that no vertices are shared.
    ///
    /// This can dramatically increase the vertex count, so make sure this is what you want.
    /// Does nothing if no [Indices] are set.
    pub fn duplicate_vertices(&mut self) {
        fn duplicate<T: Copy>(values: &[T], indices: impl Iterator<Item = usize>) -> Vec<T> {
            indices.map(|i| values[i]).collect()
        }

        let Some(indices) = self.indices.take() else {
            return;
        };

        let mesh_attributes = &mut self.attributes;

        for attributes in mesh_attributes.values_mut() {
            let indices = indices.iter();
            #[expect(
                clippy::match_same_arms,
                reason = "Although the `vec` binding on some match arms may have different types, each variant has different semantics; thus it's not guaranteed that they will use the same type forever."
            )]
            match &mut attributes.values {
                VertexAttributeValues::Float32(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint32(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint32(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Float32x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint32x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint32x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Float32x3(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint32x3(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint32x3(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint32x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint32x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Float32x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint16x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Snorm16x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint16x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Unorm16x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint16x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Snorm16x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint16x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Unorm16x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint8x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Snorm8x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint8x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Unorm8x2(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Sint8x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Snorm8x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Uint8x4(vec) => *vec = duplicate(vec, indices),
                VertexAttributeValues::Unorm8x4(vec) => *vec = duplicate(vec, indices),
            }
        }
    }

    /// Inverts the winding of the indices such that all counter-clockwise triangles are now
    /// clockwise and vice versa.
    /// For lines, their start and end indices are flipped.
    ///
    /// Does nothing if no [`Indices`] are set.
    /// If this operation succeeded, an [`Ok`] result is returned.
    pub fn invert_winding(&mut self) -> Result<(), MeshWindingInvertError> {
        fn invert<I>(
            indices: &mut [I],
            topology: PrimitiveTopology,
        ) -> Result<(), MeshWindingInvertError> {
            match topology {
                PrimitiveTopology::TriangleList => {
                    // Early return if the index count doesn't match
                    if !indices.len().is_multiple_of(3) {
                        return Err(MeshWindingInvertError::AbruptIndicesEnd);
                    }
                    for chunk in indices.chunks_mut(3) {
                        // This currently can only be optimized away with unsafe, rework this when `feature(slice_as_chunks)` gets stable.
                        let [_, b, c] = chunk else {
                            return Err(MeshWindingInvertError::AbruptIndicesEnd);
                        };
                        core::mem::swap(b, c);
                    }
                    Ok(())
                }
                PrimitiveTopology::LineList => {
                    // Early return if the index count doesn't match
                    if !indices.len().is_multiple_of(2) {
                        return Err(MeshWindingInvertError::AbruptIndicesEnd);
                    }
                    indices.reverse();
                    Ok(())
                }
                PrimitiveTopology::TriangleStrip | PrimitiveTopology::LineStrip => {
                    indices.reverse();
                    Ok(())
                }
                _ => Err(MeshWindingInvertError::WrongTopology),
            }
        }

        let topology = self.primitive_topology();
        let mesh_indices = self.indices_mut();

        match mesh_indices {
            Some(Indices::U16(vec)) => invert(vec, topology),
            Some(Indices::U32(vec)) => invert(vec, topology),
            None => Ok(()),
        }
    }

    /// Calculates the [`Mesh::ATTRIBUTE_NORMAL`] of a mesh.
    /// If the mesh is indexed, this defaults to smooth normals. Otherwise, it defaults to flat
    /// normals.
    ///
    /// # Panics
    /// Panics if [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].=
    pub fn compute_normals(&mut self) {
        let topology = self.primitive_topology();
        assert!(
            matches!(topology, PrimitiveTopology::TriangleList),
            "`compute_normals` can only work on `TriangleList`s"
        );
        if self.indices().is_none() {
            self.compute_flat_normals();
        } else {
            self.compute_smooth_normals();
        }
    }

    /// Calculates the [`Mesh::ATTRIBUTE_NORMAL`] of a mesh.
    ///
    /// # Panics
    /// Panics if [`Indices`] are set or [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].
    /// Consider calling [`Mesh::duplicate_vertices`] or exporting your mesh with normal
    /// attributes.
    ///
    /// FIXME: This should handle more cases since this is called as a part of gltf
    /// mesh loading where we can't really blame users for loading meshes that might
    /// not conform to the limitations here!
    pub fn compute_flat_normals(&mut self) {
        let topology = self.primitive_topology();
        assert!(
            self.indices().is_none(),
            "`compute_flat_normals` can't work on indexed geometry. Consider calling either `Mesh::compute_smooth_normals` or `Mesh::duplicate_vertices` followed by `Mesh::compute_flat_normals`."
        );
        assert!(
            matches!(topology, PrimitiveTopology::TriangleList),
            "`compute_flat_normals` can only work on `TriangleList`s"
        );

        let positions = self
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("`Mesh::ATTRIBUTE_POSITION` vertex attributes don't exist")
            .as_float3()
            .expect("`Mesh::ATTRIBUTE_POSITION` vertex attributes should be of type `float3`");

        let normals: Vec<_> = positions
            .chunks_exact(3)
            .map(|p| triangle_normal(p[0], p[1], p[2]))
            .flat_map(|normal| [normal; 3])
            .collect();

        self.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }

    /// Calculates the [`Mesh::ATTRIBUTE_NORMAL`] of an indexed mesh, smoothing normals for shared
    /// vertices.
    ///
    /// This method weights normals by the angles of the corners of connected triangles, thus
    /// eliminating triangle area and count as factors in the final normal. This does make it
    /// somewhat slower than [`Mesh::compute_area_weighted_normals`] which does not need to
    /// greedily normalize each triangle's normal or calculate corner angles.
    ///
    /// If you would rather have the computed normals be weighted by triangle area, see
    /// [`Mesh::compute_area_weighted_normals`] instead. If you need to weight them in some other
    /// way, see [`Mesh::compute_custom_smooth_normals`].
    ///
    /// # Panics
    /// Panics if [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].
    /// Panics if the mesh does not have indices defined.
    pub fn compute_smooth_normals(&mut self) {
        self.compute_custom_smooth_normals(|[a, b, c], positions, normals| {
            let pa = Vec3::from(positions[a]);
            let pb = Vec3::from(positions[b]);
            let pc = Vec3::from(positions[c]);

            let ab = pb - pa;
            let ba = pa - pb;
            let bc = pc - pb;
            let cb = pb - pc;
            let ca = pa - pc;
            let ac = pc - pa;

            const EPS: f32 = f32::EPSILON;
            let weight_a = if ab.length_squared() * ac.length_squared() > EPS {
                ab.angle_between(ac)
            } else {
                0.0
            };
            let weight_b = if ba.length_squared() * bc.length_squared() > EPS {
                ba.angle_between(bc)
            } else {
                0.0
            };
            let weight_c = if ca.length_squared() * cb.length_squared() > EPS {
                ca.angle_between(cb)
            } else {
                0.0
            };

            let normal = Vec3::from(triangle_normal(positions[a], positions[b], positions[c]));

            normals[a] += normal * weight_a;
            normals[b] += normal * weight_b;
            normals[c] += normal * weight_c;
        });
    }

    /// Calculates the [`Mesh::ATTRIBUTE_NORMAL`] of an indexed mesh, smoothing normals for shared
    /// vertices.
    ///
    /// This method weights normals by the area of each triangle containing the vertex. Thus,
    /// larger triangles will skew the normals of their vertices towards their own normal more
    /// than smaller triangles will.
    ///
    /// This method is actually somewhat faster than [`Mesh::compute_smooth_normals`] because an
    /// intermediate result of triangle normal calculation is already scaled by the triangle's area.
    ///
    /// If you would rather have the computed normals be influenced only by the angles of connected
    /// edges, see [`Mesh::compute_smooth_normals`] instead. If you need to weight them in some
    /// other way, see [`Mesh::compute_custom_smooth_normals`].
    ///
    /// # Panics
    /// Panics if [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].
    /// Panics if the mesh does not have indices defined.
    pub fn compute_area_weighted_normals(&mut self) {
        self.compute_custom_smooth_normals(|[a, b, c], positions, normals| {
            let normal = Vec3::from(triangle_area_normal(
                positions[a],
                positions[b],
                positions[c],
            ));
            [a, b, c].into_iter().for_each(|pos| {
                normals[pos] += normal;
            });
        });
    }

    /// Calculates the [`Mesh::ATTRIBUTE_NORMAL`] of an indexed mesh, smoothing normals for shared
    /// vertices.
    ///
    /// This method allows you to customize how normals are weighted via the `per_triangle` parameter,
    /// which must be a function or closure that accepts 3 parameters:
    /// - The indices of the three vertices of the triangle as a `[usize; 3]`.
    /// - A reference to the values of the [`Mesh::ATTRIBUTE_POSITION`] of the mesh (`&[[f32; 3]]`).
    /// - A mutable reference to the sums of all normals so far.
    ///
    /// See also the standard methods included in Bevy for calculating smooth normals:
    /// - [`Mesh::compute_smooth_normals`]
    /// - [`Mesh::compute_area_weighted_normals`]
    ///
    /// An example that would weight each connected triangle's normal equally, thus skewing normals
    /// towards the planes divided into the most triangles:
    /// ```
    /// # use bevy_asset::RenderAssetUsages;
    /// # use bevy_mesh::{Mesh, PrimitiveTopology, Meshable, MeshBuilder};
    /// # use bevy_math::{Vec3, primitives::Cuboid};
    /// # let mut mesh = Cuboid::default().mesh().build();
    /// mesh.compute_custom_smooth_normals(|[a, b, c], positions, normals| {
    ///     let normal = Vec3::from(bevy_mesh::triangle_normal(positions[a], positions[b], positions[c]));
    ///     for idx in [a, b, c] {
    ///         normals[idx] += normal;
    ///     }
    /// });
    /// ```
    ///
    /// # Panics
    /// Panics if [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].
    /// Panics if the mesh does not have indices defined.
    //
    // FIXME: This should handle more cases since this is called as a part of gltf
    // mesh loading where we can't really blame users for loading meshes that might
    // not conform to the limitations here!
    //
    // When fixed, also update "Panics" sections of
    // - [Mesh::compute_smooth_normals]
    // - [Mesh::with_computed_smooth_normals]
    // - [Mesh::compute_area_weighted_normals]
    // - [Mesh::with_computed_area_weighted_normals]
    pub fn compute_custom_smooth_normals(
        &mut self,
        mut per_triangle: impl FnMut([usize; 3], &[[f32; 3]], &mut [Vec3]),
    ) {
        let topology = self.primitive_topology();
        assert!(
            matches!(topology, PrimitiveTopology::TriangleList),
            "smooth normals can only be computed on `TriangleList`s"
        );
        let Some(indices) = self.indices() else {
            panic!("smooth normals can only be computed on indexed meshes");
        };

        let positions = self
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("`Mesh::ATTRIBUTE_POSITION` vertex attributes don't exist")
            .as_float3()
            .expect("`Mesh::ATTRIBUTE_POSITION` vertex attributes should be of type `float3`");

        let mut normals = vec![Vec3::ZERO; positions.len()];

        indices
            .iter()
            .collect::<Vec<usize>>()
            .chunks_exact(3)
            .for_each(|face| per_triangle([face[0], face[1], face[2]], positions, &mut normals));

        for normal in &mut normals {
            *normal = normal.try_normalize().unwrap_or(Vec3::ZERO);
        }

        self.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }

    /// Generate tangents for the mesh using the `mikktspace` algorithm.
    ///
    /// Sets the [`Mesh::ATTRIBUTE_TANGENT`] attribute if successful.
    /// Requires a [`PrimitiveTopology::TriangleList`] topology and the [`Mesh::ATTRIBUTE_POSITION`], [`Mesh::ATTRIBUTE_NORMAL`] and [`Mesh::ATTRIBUTE_UV_0`] attributes set.
    #[cfg(feature = "bevy_mikktspace")]
    pub fn generate_tangents(&mut self) -> Result<(), super::GenerateTangentsError> {
        let topology = self.primitive_topology();
        let tangents = super::generate_tangents_for_mesh(self, topology)?;
        self.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
        Ok(())
    }

    /// Merges the [`Mesh`] data of `other` with `self`. The attributes and indices of `other` will be appended to `self`.
    ///
    /// Note that attributes of `other` that don't exist on `self` will be ignored.
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// # Errors
    ///
    /// If any of the following conditions are not met, this function errors:
    /// * All of the vertex attributes that have the same attribute id, must also
    ///   have the same attribute type.
    ///   For example two attributes with the same id, but where one is a
    ///   [`VertexAttributeValues::Float32`] and the other is a
    ///   [`VertexAttributeValues::Float32x3`], would be invalid.
    pub fn merge(&mut self, other: &MeshExtractableData) -> Result<(), MeshMergeError> {
        use VertexAttributeValues::*;

        // The indices of `other` should start after the last vertex of `self`.
        let index_offset = self.count_vertices();

        // Extend attributes of `self` with attributes of `other`.
        for (attribute, values) in self.attributes_mut() {
            if let Some(other_values) = other.attribute(attribute.id) {
                #[expect(
                    clippy::match_same_arms,
                    reason = "Although the bindings on some match arms may have different types, each variant has different semantics; thus it's not guaranteed that they will use the same type forever."
                )]
                match (values, other_values) {
                    (Float32(vec1), Float32(vec2)) => vec1.extend(vec2),
                    (Sint32(vec1), Sint32(vec2)) => vec1.extend(vec2),
                    (Uint32(vec1), Uint32(vec2)) => vec1.extend(vec2),
                    (Float32x2(vec1), Float32x2(vec2)) => vec1.extend(vec2),
                    (Sint32x2(vec1), Sint32x2(vec2)) => vec1.extend(vec2),
                    (Uint32x2(vec1), Uint32x2(vec2)) => vec1.extend(vec2),
                    (Float32x3(vec1), Float32x3(vec2)) => vec1.extend(vec2),
                    (Sint32x3(vec1), Sint32x3(vec2)) => vec1.extend(vec2),
                    (Uint32x3(vec1), Uint32x3(vec2)) => vec1.extend(vec2),
                    (Sint32x4(vec1), Sint32x4(vec2)) => vec1.extend(vec2),
                    (Uint32x4(vec1), Uint32x4(vec2)) => vec1.extend(vec2),
                    (Float32x4(vec1), Float32x4(vec2)) => vec1.extend(vec2),
                    (Sint16x2(vec1), Sint16x2(vec2)) => vec1.extend(vec2),
                    (Snorm16x2(vec1), Snorm16x2(vec2)) => vec1.extend(vec2),
                    (Uint16x2(vec1), Uint16x2(vec2)) => vec1.extend(vec2),
                    (Unorm16x2(vec1), Unorm16x2(vec2)) => vec1.extend(vec2),
                    (Sint16x4(vec1), Sint16x4(vec2)) => vec1.extend(vec2),
                    (Snorm16x4(vec1), Snorm16x4(vec2)) => vec1.extend(vec2),
                    (Uint16x4(vec1), Uint16x4(vec2)) => vec1.extend(vec2),
                    (Unorm16x4(vec1), Unorm16x4(vec2)) => vec1.extend(vec2),
                    (Sint8x2(vec1), Sint8x2(vec2)) => vec1.extend(vec2),
                    (Snorm8x2(vec1), Snorm8x2(vec2)) => vec1.extend(vec2),
                    (Uint8x2(vec1), Uint8x2(vec2)) => vec1.extend(vec2),
                    (Unorm8x2(vec1), Unorm8x2(vec2)) => vec1.extend(vec2),
                    (Sint8x4(vec1), Sint8x4(vec2)) => vec1.extend(vec2),
                    (Snorm8x4(vec1), Snorm8x4(vec2)) => vec1.extend(vec2),
                    (Uint8x4(vec1), Uint8x4(vec2)) => vec1.extend(vec2),
                    (Unorm8x4(vec1), Unorm8x4(vec2)) => vec1.extend(vec2),
                    _ => {
                        return Err(MeshMergeError::IncompatibleVertexAttributes {
                            self_attribute: *attribute,
                            other_attribute: other
                                .attribute_data(attribute.id)
                                .map(|data| data.attribute),
                        })
                    }
                }
            }
        }

        // Extend indices of `self` with indices of `other`.
        if let (Some(indices), Some(other_indices)) = (self.indices_mut(), other.indices()) {
            indices.extend(other_indices.iter().map(|i| (i + index_offset) as u32));
        }
        Ok(())
    }

    /// Transforms the vertex positions, normals, and tangents of the mesh in place by the given [`Transform`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    pub fn transform_by(&mut self, transform: Transform) {
        // Needed when transforming normals and tangents
        let scale_recip = 1. / transform.scale;
        debug_assert!(
            transform.scale.yzx() * transform.scale.zxy() != Vec3::ZERO,
            "mesh transform scale cannot be zero on more than one axis"
        );

        if let Some(VertexAttributeValues::Float32x3(positions)) =
            self.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            // Apply scale, rotation, and translation to vertex positions
            positions
                .iter_mut()
                .for_each(|pos| *pos = transform.transform_point(Vec3::from_slice(pos)).to_array());
        }

        // No need to transform normals or tangents if rotation is near identity and scale is uniform
        if transform.rotation.is_near_identity()
            && transform.scale.x == transform.scale.y
            && transform.scale.y == transform.scale.z
        {
            return;
        }

        if let Some(VertexAttributeValues::Float32x3(normals)) =
            self.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
        {
            // Transform normals, taking into account non-uniform scaling and rotation
            normals.iter_mut().for_each(|normal| {
                *normal = (transform.rotation
                    * scale_normal(Vec3::from_array(*normal), scale_recip))
                .to_array();
            });
        }

        if let Some(VertexAttributeValues::Float32x4(tangents)) =
            self.attribute_mut(Mesh::ATTRIBUTE_TANGENT)
        {
            // Transform tangents, taking into account non-uniform scaling and rotation
            tangents.iter_mut().for_each(|tangent| {
                let handedness = tangent[3];
                let scaled_tangent = Vec3::from_slice(tangent) * transform.scale;
                *tangent = (transform.rotation * scaled_tangent.normalize_or_zero())
                    .extend(handedness)
                    .to_array();
            });
        }
    }

    /// Translates the vertex positions of the mesh in place by the given [`Vec3`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    pub fn translate_by(&mut self, translation: Vec3) {
        if translation == Vec3::ZERO {
            return;
        }

        if let Some(VertexAttributeValues::Float32x3(positions)) =
            self.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            // Apply translation to vertex positions
            positions
                .iter_mut()
                .for_each(|pos| *pos = (Vec3::from_slice(pos) + translation).to_array());
        }
    }

    /// Rotates the vertex positions, normals, and tangents of the mesh in place by the given [`Quat`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    pub fn rotate_by(&mut self, rotation: Quat) {
        if let Some(VertexAttributeValues::Float32x3(positions)) =
            self.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            // Apply rotation to vertex positions
            positions
                .iter_mut()
                .for_each(|pos| *pos = (rotation * Vec3::from_slice(pos)).to_array());
        }

        // No need to transform normals or tangents if rotation is near identity
        if rotation.is_near_identity() {
            return;
        }

        if let Some(VertexAttributeValues::Float32x3(normals)) =
            self.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
        {
            // Transform normals
            normals.iter_mut().for_each(|normal| {
                *normal = (rotation * Vec3::from_slice(normal).normalize_or_zero()).to_array();
            });
        }

        if let Some(VertexAttributeValues::Float32x4(tangents)) =
            self.attribute_mut(Mesh::ATTRIBUTE_TANGENT)
        {
            // Transform tangents
            tangents.iter_mut().for_each(|tangent| {
                let handedness = tangent[3];
                *tangent = (rotation * Vec3::from_slice(tangent).normalize_or_zero())
                    .extend(handedness)
                    .to_array();
            });
        }
    }

    /// Scales the vertex positions, normals, and tangents of the mesh in place by the given [`Vec3`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    pub fn scale_by(&mut self, scale: Vec3) {
        // Needed when transforming normals and tangents
        let scale_recip = 1. / scale;
        debug_assert!(
            scale.yzx() * scale.zxy() != Vec3::ZERO,
            "mesh transform scale cannot be zero on more than one axis"
        );

        if let Some(VertexAttributeValues::Float32x3(positions)) =
            self.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            // Apply scale to vertex positions
            positions
                .iter_mut()
                .for_each(|pos| *pos = (scale * Vec3::from_slice(pos)).to_array());
        }

        // No need to transform normals or tangents if scale is uniform
        if scale.x == scale.y && scale.y == scale.z {
            return;
        }

        if let Some(VertexAttributeValues::Float32x3(normals)) =
            self.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
        {
            // Transform normals, taking into account non-uniform scaling
            normals.iter_mut().for_each(|normal| {
                *normal = scale_normal(Vec3::from_array(*normal), scale_recip).to_array();
            });
        }

        if let Some(VertexAttributeValues::Float32x4(tangents)) =
            self.attribute_mut(Mesh::ATTRIBUTE_TANGENT)
        {
            // Transform tangents, taking into account non-uniform scaling
            tangents.iter_mut().for_each(|tangent| {
                let handedness = tangent[3];
                let scaled_tangent = Vec3::from_slice(tangent) * scale;
                *tangent = scaled_tangent
                    .normalize_or_zero()
                    .extend(handedness)
                    .to_array();
            });
        }
    }

    /// Normalize joint weights so they sum to 1.
    pub fn normalize_joint_weights(&mut self) -> Result<(), ExtractableAssetAccessError> {
        if let Some(VertexAttributeValues::Float32x4(joints)) =
            self.attribute_mut(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            for weights in joints.iter_mut() {
                // force negative weights to zero
                weights.iter_mut().for_each(|w| *w = w.max(0.0));

                let sum: f32 = weights.iter().sum();
                if sum == 0.0 {
                    // all-zero weights are invalid
                    weights[0] = 1.0;
                } else {
                    let recip = sum.recip();
                    for weight in weights.iter_mut() {
                        *weight *= recip;
                    }
                }
            }
        }

        Ok(())
    }

    /// Get a list of this Mesh's [triangles] as an iterator if possible.
    ///
    /// Returns an error if any of the following conditions are met (see [`MeshTrianglesError`]):
    /// * The Mesh's [primitive topology] is not `TriangleList` or `TriangleStrip`.
    /// * The Mesh is missing position or index data.
    /// * The Mesh's position data has the wrong format (not `Float32x3`).
    ///
    /// [primitive topology]: PrimitiveTopology
    /// [triangles]: Triangle3d
    pub fn triangles(
        &self,
        topology: PrimitiveTopology,
    ) -> Result<impl Iterator<Item = Triangle3d> + '_, MeshTrianglesError> {
        let Some(position_data) = self.attribute(Mesh::ATTRIBUTE_POSITION) else {
            return Err(MeshTrianglesError::BadPositions);
        };

        let Some(vertices) = position_data.as_float3() else {
            return Err(MeshTrianglesError::PositionsFormat);
        };

        let Some(indices) = self.indices() else {
            return Err(MeshTrianglesError::BadIndices);
        };

        match topology {
            PrimitiveTopology::TriangleList => {
                // When indices reference out-of-bounds vertex data, the triangle is omitted.
                // This implicitly truncates the indices to a multiple of 3.
                let iterator = match indices {
                    Indices::U16(vec) => FourIterators::First(
                        vec.as_slice()
                            .chunks_exact(3)
                            .flat_map(move |indices| indices_to_triangle(vertices, indices)),
                    ),
                    Indices::U32(vec) => FourIterators::Second(
                        vec.as_slice()
                            .chunks_exact(3)
                            .flat_map(move |indices| indices_to_triangle(vertices, indices)),
                    ),
                };

                return Ok(iterator);
            }

            PrimitiveTopology::TriangleStrip => {
                // When indices reference out-of-bounds vertex data, the triangle is omitted.
                // If there aren't enough indices to make a triangle, then an empty vector will be
                // returned.
                let iterator = match indices {
                    Indices::U16(vec) => {
                        FourIterators::Third(vec.as_slice().windows(3).enumerate().flat_map(
                            move |(i, indices)| {
                                if i % 2 == 0 {
                                    indices_to_triangle(vertices, indices)
                                } else {
                                    indices_to_triangle(
                                        vertices,
                                        &[indices[1], indices[0], indices[2]],
                                    )
                                }
                            },
                        ))
                    }
                    Indices::U32(vec) => {
                        FourIterators::Fourth(vec.as_slice().windows(3).enumerate().flat_map(
                            move |(i, indices)| {
                                if i % 2 == 0 {
                                    indices_to_triangle(vertices, indices)
                                } else {
                                    indices_to_triangle(
                                        vertices,
                                        &[indices[1], indices[0], indices[2]],
                                    )
                                }
                            },
                        ))
                    }
                };

                return Ok(iterator);
            }

            _ => {
                return Err(MeshTrianglesError::WrongTopology);
            }
        };

        fn indices_to_triangle<T: TryInto<usize> + Copy>(
            vertices: &[[f32; 3]],
            indices: &[T],
        ) -> Option<Triangle3d> {
            let vert0: Vec3 = Vec3::from(*vertices.get(indices[0].try_into().ok()?)?);
            let vert1: Vec3 = Vec3::from(*vertices.get(indices[1].try_into().ok()?)?);
            let vert2: Vec3 = Vec3::from(*vertices.get(indices[2].try_into().ok()?)?);
            Some(Triangle3d {
                vertices: [vert0, vert1, vert2],
            })
        }
    }

    /// Consumes the mesh and returns a mesh with data set for a vertex attribute (position, normal, etc.).
    /// The name will often be one of the associated constants such as [`Mesh::ATTRIBUTE_POSITION`].
    ///
    /// (Alternatively, you can use [`Mesh::insert_attribute`] to mutate an existing mesh in-place)
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    #[must_use]
    #[inline]
    pub fn with_inserted_attribute(
        mut self,
        attribute: MeshVertexAttribute,
        values: impl Into<VertexAttributeValues>,
    ) -> Self {
        self.insert_attribute(attribute, values);
        self
    }

    /// Consumes the mesh and returns a mesh without the data for a vertex attribute
    ///
    /// (Alternatively, you can use [`Mesh::remove_attribute`] to mutate an existing mesh in-place)
    #[must_use]
    pub fn with_removed_attribute(mut self, attribute: impl Into<MeshVertexAttributeId>) -> Self {
        self.remove_attribute(attribute);
        self
    }

    /// Consumes the mesh and returns a mesh with the given vertex indices. They describe how triangles
    /// are constructed out of the vertex attributes and are therefore only useful for the
    /// [`PrimitiveTopology`] variants that use triangles.
    ///
    /// (Alternatively, you can use [`Mesh::insert_indices`] to mutate an existing mesh in-place)
    #[must_use]
    #[inline]
    pub fn with_inserted_indices(mut self, indices: Indices) -> Self {
        self.insert_indices(indices);
        self
    }

    /// Consumes the mesh and returns a mesh without the vertex `indices` of the mesh.
    ///
    /// (Alternatively, you can use [`Mesh::remove_indices`] to mutate an existing mesh in-place)
    #[must_use]
    pub fn with_removed_indices(mut self) -> Self {
        self.remove_indices();
        self
    }

    /// Consumes the mesh and returns a mesh with no shared vertices.
    ///
    /// This can dramatically increase the vertex count, so make sure this is what you want.
    /// Does nothing if no [`Indices`] are set.
    ///
    /// (Alternatively, you can use [`Mesh::duplicate_vertices`] to mutate an existing mesh in-place)
    #[must_use]
    pub fn with_duplicated_vertices(mut self) -> Self {
        self.duplicate_vertices();
        self
    }

    /// Consumes the mesh and returns a mesh with inverted winding of the indices such
    /// that all counter-clockwise triangles are now clockwise and vice versa.
    ///
    /// Does nothing if no [`Indices`] are set.
    pub fn with_inverted_winding(mut self) -> Result<Self, MeshWindingInvertError> {
        self.invert_winding().map(|_| self)
    }

    /// Consumes the mesh and returns a mesh with calculated [`Mesh::ATTRIBUTE_NORMAL`].
    /// If the mesh is indexed, this defaults to smooth normals. Otherwise, it defaults to flat
    /// normals.
    ///
    /// (Alternatively, you can use [`Mesh::compute_normals`] to mutate an existing mesh in-place)
    ///
    /// # Panics
    /// Panics if [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].
    #[must_use]
    pub fn with_computed_normals(mut self) -> Self {
        self.compute_normals();
        self
    }

    /// Consumes the mesh and returns a mesh with calculated [`Mesh::ATTRIBUTE_NORMAL`].
    ///
    /// (Alternatively, you can use [`Mesh::compute_flat_normals`] to mutate an existing mesh in-place)
    ///
    /// # Panics
    /// Panics if [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].
    /// Panics if the mesh has indices defined
    pub fn with_computed_flat_normals(mut self) -> Self {
        self.compute_flat_normals();
        self
    }

    /// Consumes the mesh and returns a mesh with calculated [`Mesh::ATTRIBUTE_NORMAL`].
    ///
    /// (Alternatively, you can use [`Mesh::compute_smooth_normals`] to mutate an existing mesh in-place)
    ///
    /// This method weights normals by the angles of triangle corners connected to each vertex. If
    /// you would rather have the computed normals be weighted by triangle area, see
    /// [`Mesh::with_computed_area_weighted_normals`] instead.
    ///
    /// # Panics
    /// Panics if [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].
    /// Panics if the mesh does not have indices defined.
    pub fn with_computed_smooth_normals(mut self) -> Self {
        self.compute_smooth_normals();
        self
    }

    /// Consumes the mesh and returns a mesh with calculated [`Mesh::ATTRIBUTE_NORMAL`].
    ///
    /// (Alternatively, you can use [`Mesh::compute_area_weighted_normals`] to mutate an existing mesh in-place)
    ///
    /// This method weights normals by the area of each triangle containing the vertex. Thus,
    /// larger triangles will skew the normals of their vertices towards their own normal more
    /// than smaller triangles will. If you would rather have the computed normals be influenced
    /// only by the angles of connected edges, see [`Mesh::with_computed_smooth_normals`] instead.
    ///
    /// # Panics
    /// Panics if [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].
    /// Panics if the mesh does not have indices defined.
    pub fn with_computed_area_weighted_normals(mut self) -> Self {
        self.compute_area_weighted_normals();
        self
    }

    /// Consumes the mesh and returns a mesh with tangents generated using the `mikktspace` algorithm.
    ///
    /// The resulting mesh will have the [`Mesh::ATTRIBUTE_TANGENT`] attribute if successful.
    ///
    /// (Alternatively, you can use [`Mesh::generate_tangents`] to mutate an existing mesh in-place)
    ///
    /// Requires a [`PrimitiveTopology::TriangleList`] topology and the [`Mesh::ATTRIBUTE_POSITION`], [`Mesh::ATTRIBUTE_NORMAL`] and [`Mesh::ATTRIBUTE_UV_0`] attributes set.
    #[cfg(feature = "bevy_mikktspace")]
    pub fn with_generated_tangents(mut self) -> Result<Mesh, super::GenerateTangentsError> {
        self.generate_tangents()?;
        Ok(self)
    }
}

#[cfg(feature = "morph")]
impl MeshExtractableData {
    /// Whether this mesh has morph targets.
    pub fn has_morph_targets(&self) -> bool {
        self.morph_targets.is_some()
    }

    /// Set [morph targets] image for this mesh. This requires a "morph target image". See [`MorphTargetImage`](crate::morph::MorphTargetImage) for info.
    ///
    /// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
    pub fn set_morph_targets(&mut self, morph_targets: Handle<Image>) {
        self.morph_targets.replace(morph_targets);
    }

    /// Retrieve the morph targets for this mesh, or None if there are no morph targets.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`or
    /// if the morph targets do not exist.
    pub fn morph_targets(&self) -> Option<&Handle<Image>> {
        self.morph_targets.as_ref()
    }

    /// Sets the names of each morph target. This should correspond to the order of the morph targets in `set_morph_targets`.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    pub fn set_morph_target_names(&mut self, names: Vec<String>) {
        self.morph_target_names.replace(names);
    }

    /// Gets a list of all morph target names, if they exist.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`or
    /// if the morph targets do not exist.
    pub fn morph_target_names(&self) -> Option<&[String]> {
        self.morph_target_names.as_deref()
    }

    /// Consumes the mesh and returns a mesh with the given [morph targets].
    ///
    /// This requires a "morph target image". See [`MorphTargetImage`](crate::morph::MorphTargetImage) for info.
    ///
    /// (Alternatively, you can use [`Mesh::set_morph_targets`] to mutate an existing mesh in-place)
    ///
    /// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
    #[must_use]
    pub fn with_morph_targets(mut self, morph_targets: Handle<Image>) -> Self {
        self.set_morph_targets(morph_targets);
        self
    }

    /// Consumes the mesh and returns a mesh with morph target names.
    /// Names should correspond to the order of the morph targets in `set_morph_targets`.
    ///
    /// (Alternatively, you can use [`Mesh::set_morph_target_names`] to mutate an existing mesh in-place)
    #[must_use]
    pub fn with_morph_target_names(mut self, names: Vec<String>) -> Self {
        self.set_morph_target_names(names);
        self
    }
}

/// Correctly scales and renormalizes an already normalized `normal` by the scale determined by its reciprocal `scale_recip`
pub(crate) fn scale_normal(normal: Vec3, scale_recip: Vec3) -> Vec3 {
    // This is basically just `normal * scale_recip` but with the added rule that `0. * anything == 0.`
    // This is necessary because components of `scale_recip` may be infinities, which do not multiply to zero
    let n = Vec3::select(normal.cmpeq(Vec3::ZERO), Vec3::ZERO, normal * scale_recip);

    // If n is finite, no component of `scale_recip` was infinite or the normal was perpendicular to the scale
    // else the scale had at least one zero-component and the normal needs to point along the direction of that component
    if n.is_finite() {
        n.normalize_or_zero()
    } else {
        Vec3::select(n.abs().cmpeq(Vec3::INFINITY), n.signum(), Vec3::ZERO).normalize()
    }
}
