use bevy_transform::components::Transform;
pub use wgpu_types::PrimitiveTopology;

use super::{
    skinning::{SkinnedMeshBounds, SkinnedMeshBoundsError},
    triangle_area_normal, triangle_normal, FourIterators, Indices, MeshAttributeData,
    MeshTrianglesError, MeshVertexAttribute, MeshVertexAttributeId, MeshVertexBufferLayout,
    MeshVertexBufferLayoutRef, MeshVertexBufferLayouts, MeshWindingInvertError,
    VertexAttributeValues, VertexBufferLayout,
};
#[cfg(feature = "serialize")]
use crate::SerializedMeshAttributeData;
use alloc::collections::BTreeMap;
#[cfg(feature = "morph")]
use bevy_asset::Handle;
use bevy_asset::{Asset, RenderAssetUsages};
#[cfg(feature = "morph")]
use bevy_image::Image;
use bevy_math::{bounding::Aabb3d, primitives::Triangle3d, *};
#[cfg(feature = "serialize")]
use bevy_platform::collections::HashMap;
use bevy_reflect::Reflect;
use bevy_utils::hashbrown::hash_map;
use bytemuck::cast_slice;
#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::warn;
use core::hash::{Hash, Hasher};
use core::ptr;
use wgpu_types::{VertexAttribute, VertexFormat, VertexStepMode};

pub const INDEX_BUFFER_ASSET_INDEX: u64 = 0;
pub const VERTEX_ATTRIBUTE_BUFFER_ID: u64 = 10;

/// Error from accessing mesh vertex attributes or indices
#[derive(Error, Debug, Clone)]
pub enum MeshAccessError {
    #[error("The mesh vertex/index data has been extracted to the RenderWorld (via `Mesh::asset_usage`)")]
    ExtractedToRenderWorld,
    #[error("The requested mesh data wasn't found in this mesh")]
    NotFound,
}

const MESH_EXTRACTED_ERROR: &str = "Mesh has been extracted to RenderWorld. To access vertex attributes, the mesh `asset_usage` must include `MAIN_WORLD`";

// storage for extractable data with access methods which return errors if the
// contents have already been extracted
#[derive(Debug, Clone, PartialEq, Reflect, Default)]
enum MeshExtractableData<T> {
    Data(T),
    #[default]
    NoData,
    ExtractedToRenderWorld,
}

impl<T> MeshExtractableData<T> {
    // get a reference to internal data. returns error if data has been extracted, or if no
    // data exists
    fn as_ref(&self) -> Result<&T, MeshAccessError> {
        match self {
            MeshExtractableData::Data(data) => Ok(data),
            MeshExtractableData::NoData => Err(MeshAccessError::NotFound),
            MeshExtractableData::ExtractedToRenderWorld => {
                Err(MeshAccessError::ExtractedToRenderWorld)
            }
        }
    }

    // get an optional reference to internal data. returns error if data has been extracted
    fn as_ref_option(&self) -> Result<Option<&T>, MeshAccessError> {
        match self {
            MeshExtractableData::Data(data) => Ok(Some(data)),
            MeshExtractableData::NoData => Ok(None),
            MeshExtractableData::ExtractedToRenderWorld => {
                Err(MeshAccessError::ExtractedToRenderWorld)
            }
        }
    }

    // get a mutable reference to internal data. returns error if data has been extracted,
    // or if no data exists
    fn as_mut(&mut self) -> Result<&mut T, MeshAccessError> {
        match self {
            MeshExtractableData::Data(data) => Ok(data),
            MeshExtractableData::NoData => Err(MeshAccessError::NotFound),
            MeshExtractableData::ExtractedToRenderWorld => {
                Err(MeshAccessError::ExtractedToRenderWorld)
            }
        }
    }

    // get an optional mutable reference to internal data. returns error if data has been extracted
    fn as_mut_option(&mut self) -> Result<Option<&mut T>, MeshAccessError> {
        match self {
            MeshExtractableData::Data(data) => Ok(Some(data)),
            MeshExtractableData::NoData => Ok(None),
            MeshExtractableData::ExtractedToRenderWorld => {
                Err(MeshAccessError::ExtractedToRenderWorld)
            }
        }
    }

    // extract data and replace self with `ExtractedToRenderWorld`. returns error if
    // data has been extracted
    fn extract(&mut self) -> Result<MeshExtractableData<T>, MeshAccessError> {
        match core::mem::replace(self, MeshExtractableData::ExtractedToRenderWorld) {
            MeshExtractableData::ExtractedToRenderWorld => {
                Err(MeshAccessError::ExtractedToRenderWorld)
            }
            not_extracted => Ok(not_extracted),
        }
    }

    // replace internal data. returns the existing data, or an error if data has been extracted
    fn replace(
        &mut self,
        data: impl Into<MeshExtractableData<T>>,
    ) -> Result<Option<T>, MeshAccessError> {
        match core::mem::replace(self, data.into()) {
            MeshExtractableData::ExtractedToRenderWorld => {
                *self = MeshExtractableData::ExtractedToRenderWorld;
                Err(MeshAccessError::ExtractedToRenderWorld)
            }
            MeshExtractableData::Data(t) => Ok(Some(t)),
            MeshExtractableData::NoData => Ok(None),
        }
    }
}

impl<T> From<Option<T>> for MeshExtractableData<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(data) => MeshExtractableData::Data(data),
            None => MeshExtractableData::NoData,
        }
    }
}

/// A 3D object made out of vertices representing triangles, lines, or points,
/// with "attribute" values for each vertex.
///
/// Meshes can be automatically generated by a bevy `AssetLoader` (generally by loading a `Gltf` file),
/// or by converting a [primitive](bevy_math::primitives) using [`into`](Into).
/// It is also possible to create one manually. They can be edited after creation.
///
/// Meshes can be rendered with a [`Mesh2d`](crate::Mesh2d) and `MeshMaterial2d`
/// or [`Mesh3d`](crate::Mesh3d) and `MeshMaterial3d` for 2D and 3D respectively.
///
/// A [`Mesh`] in Bevy is equivalent to a "primitive" in the glTF format, for a
/// glTF Mesh representation, see `GltfMesh`.
///
/// ## Manual creation
///
/// The following function will construct a flat mesh, to be rendered with a
/// `StandardMaterial` or `ColorMaterial`:
///
/// ```
/// # use bevy_mesh::{Mesh, Indices, PrimitiveTopology};
/// # use bevy_asset::RenderAssetUsages;
/// fn create_simple_parallelogram() -> Mesh {
///     // Create a new mesh using a triangle list topology, where each set of 3 vertices composes a triangle.
///     Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
///         // Add 4 vertices, each with its own position attribute (coordinate in
///         // 3D space), for each of the corners of the parallelogram.
///         .with_inserted_attribute(
///             Mesh::ATTRIBUTE_POSITION,
///             vec![[0.0, 0.0, 0.0], [1.0, 2.0, 0.0], [2.0, 2.0, 0.0], [1.0, 0.0, 0.0]]
///         )
///         // Assign a UV coordinate to each vertex.
///         .with_inserted_attribute(
///             Mesh::ATTRIBUTE_UV_0,
///             vec![[0.0, 1.0], [0.5, 0.0], [1.0, 0.0], [0.5, 1.0]]
///         )
///         // Assign normals (everything points outwards)
///         .with_inserted_attribute(
///             Mesh::ATTRIBUTE_NORMAL,
///             vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]
///         )
///         // After defining all the vertices and their attributes, build each triangle using the
///         // indices of the vertices that make it up in a counter-clockwise order.
///         .with_inserted_indices(Indices::U32(vec![
///             // First triangle
///             0, 3, 1,
///             // Second triangle
///             1, 3, 2
///         ]))
/// }
/// ```
///
/// You can see how it looks like [here](https://github.com/bevyengine/bevy/blob/main/assets/docs/Mesh.png),
/// used in a [`Mesh3d`](crate::Mesh3d) with a square bevy logo texture, with added axis, points,
/// lines and text for clarity.
///
/// ## Other examples
///
/// For further visualization, explanation, and examples, see the built-in Bevy examples,
/// and the [implementation of the built-in shapes](https://github.com/bevyengine/bevy/tree/main/crates/bevy_mesh/src/primitives).
/// In particular, [generate_custom_mesh](https://github.com/bevyengine/bevy/blob/main/examples/3d/generate_custom_mesh.rs)
/// teaches you to access and modify the attributes of a [`Mesh`] after creating it.
///
/// ## Common points of confusion
///
/// - UV maps in Bevy start at the top-left, see [`ATTRIBUTE_UV_0`](Mesh::ATTRIBUTE_UV_0),
///   other APIs can have other conventions, `OpenGL` starts at bottom-left.
/// - It is possible and sometimes useful for multiple vertices to have the same
///   [position attribute](Mesh::ATTRIBUTE_POSITION) value,
///   it's a common technique in 3D modeling for complex UV mapping or other calculations.
/// - Bevy performs frustum culling based on the `Aabb` of meshes, which is calculated
///   and added automatically for new meshes only. If a mesh is modified, the entity's `Aabb`
///   needs to be updated manually or deleted so that it is re-calculated.
///
/// ## Use with `StandardMaterial`
///
/// To render correctly with `StandardMaterial`, a mesh needs to have properly defined:
/// - [`UVs`](Mesh::ATTRIBUTE_UV_0): Bevy needs to know how to map a texture onto the mesh
///   (also true for `ColorMaterial`).
/// - [`Normals`](Mesh::ATTRIBUTE_NORMAL): Bevy needs to know how light interacts with your mesh.
///   [0.0, 0.0, 1.0] is very common for simple flat meshes on the XY plane,
///   because simple meshes are smooth and they don't require complex light calculations.
/// - Vertex winding order: by default, `StandardMaterial.cull_mode` is `Some(Face::Back)`,
///   which means that Bevy would *only* render the "front" of each triangle, which
///   is the side of the triangle from where the vertices appear in a *counter-clockwise* order.
///
/// ## Remote Inspection
///
/// To transmit a [`Mesh`] between two running Bevy apps, e.g. through BRP, use [`SerializedMesh`].
/// This type is only meant for short-term transmission between same versions and should not be stored anywhere.
#[derive(Asset, Debug, Clone, Reflect, PartialEq)]
#[reflect(Clone)]
pub struct Mesh {
    #[reflect(ignore, clone)]
    primitive_topology: PrimitiveTopology,
    /// `std::collections::BTreeMap` with all defined vertex attributes (Positions, Normals, ...)
    /// for this mesh. Attribute ids to attribute values.
    /// Uses a [`BTreeMap`] because, unlike `HashMap`, it has a defined iteration order,
    /// which allows easy stable `VertexBuffers` (i.e. same buffer order)
    #[reflect(ignore, clone)]
    attributes: MeshExtractableData<BTreeMap<MeshVertexAttributeId, MeshAttributeData>>,
    indices: MeshExtractableData<Indices>,
    #[cfg(feature = "morph")]
    morph_targets: MeshExtractableData<Handle<Image>>,
    #[cfg(feature = "morph")]
    morph_target_names: MeshExtractableData<Vec<String>>,
    pub asset_usage: RenderAssetUsages,
    /// Whether or not to build a BLAS for use with `bevy_solari` raytracing.
    ///
    /// Note that this is _not_ whether the mesh is _compatible_ with `bevy_solari` raytracing.
    /// This field just controls whether or not a BLAS gets built for this mesh, assuming that
    /// the mesh is compatible.
    ///
    /// The use case for this field is using lower-resolution proxy meshes for raytracing (to save on BLAS memory usage),
    /// while using higher-resolution meshes for raster. You can set this field to true for the lower-resolution proxy mesh,
    /// and to false for the high-resolution raster mesh.
    ///
    /// Alternatively, you can use the same mesh for both raster and raytracing, with this field set to true.
    ///
    /// Does nothing if not used with `bevy_solari`, or if the mesh is not compatible
    /// with `bevy_solari` (see `bevy_solari`'s docs).
    pub enable_raytracing: bool,
    /// Precomputed min and max extents of the mesh position data. Used mainly for constructing `Aabb`s for frustum culling.
    /// This data will be set if/when a mesh is extracted to the GPU
    pub final_aabb: Option<Aabb3d>,
    skinned_mesh_bounds: Option<SkinnedMeshBounds>,
}

impl Mesh {
    /// Where the vertex is located in space. Use in conjunction with [`Mesh::insert_attribute`]
    /// or [`Mesh::with_inserted_attribute`].
    ///
    /// The format of this attribute is [`VertexFormat::Float32x3`].
    pub const ATTRIBUTE_POSITION: MeshVertexAttribute =
        MeshVertexAttribute::new("Vertex_Position", 0, VertexFormat::Float32x3);

    /// The direction the vertex normal is facing in.
    /// Use in conjunction with [`Mesh::insert_attribute`] or [`Mesh::with_inserted_attribute`].
    ///
    /// The format of this attribute is [`VertexFormat::Float32x3`].
    pub const ATTRIBUTE_NORMAL: MeshVertexAttribute =
        MeshVertexAttribute::new("Vertex_Normal", 1, VertexFormat::Float32x3);

    /// Texture coordinates for the vertex. Use in conjunction with [`Mesh::insert_attribute`]
    /// or [`Mesh::with_inserted_attribute`].
    ///
    /// Generally `[0.,0.]` is mapped to the top left of the texture, and `[1.,1.]` to the bottom-right.
    ///
    /// By default values outside will be clamped per pixel not for the vertex,
    /// "stretching" the borders of the texture.
    /// This behavior can be useful in some cases, usually when the borders have only
    /// one color, for example a logo, and you want to "extend" those borders.
    ///
    /// For different mapping outside of `0..=1` range,
    /// see [`ImageAddressMode`](bevy_image::ImageAddressMode).
    ///
    /// The format of this attribute is [`VertexFormat::Float32x2`].
    pub const ATTRIBUTE_UV_0: MeshVertexAttribute =
        MeshVertexAttribute::new("Vertex_Uv", 2, VertexFormat::Float32x2);

    /// Alternate texture coordinates for the vertex. Use in conjunction with
    /// [`Mesh::insert_attribute`] or [`Mesh::with_inserted_attribute`].
    ///
    /// Typically, these are used for lightmaps, textures that provide
    /// precomputed illumination.
    ///
    /// The format of this attribute is [`VertexFormat::Float32x2`].
    pub const ATTRIBUTE_UV_1: MeshVertexAttribute =
        MeshVertexAttribute::new("Vertex_Uv_1", 3, VertexFormat::Float32x2);

    /// The direction of the vertex tangent. Used for normal mapping.
    /// Usually generated with [`generate_tangents`](Mesh::generate_tangents) or
    /// [`with_generated_tangents`](Mesh::with_generated_tangents).
    ///
    /// The format of this attribute is [`VertexFormat::Float32x4`].
    pub const ATTRIBUTE_TANGENT: MeshVertexAttribute =
        MeshVertexAttribute::new("Vertex_Tangent", 4, VertexFormat::Float32x4);

    /// Per vertex coloring. Use in conjunction with [`Mesh::insert_attribute`]
    /// or [`Mesh::with_inserted_attribute`].
    ///
    /// The format of this attribute is [`VertexFormat::Float32x4`].
    pub const ATTRIBUTE_COLOR: MeshVertexAttribute =
        MeshVertexAttribute::new("Vertex_Color", 5, VertexFormat::Float32x4);

    /// Per vertex joint transform matrix weight. Use in conjunction with [`Mesh::insert_attribute`]
    /// or [`Mesh::with_inserted_attribute`].
    ///
    /// The format of this attribute is [`VertexFormat::Float32x4`].
    pub const ATTRIBUTE_JOINT_WEIGHT: MeshVertexAttribute =
        MeshVertexAttribute::new("Vertex_JointWeight", 6, VertexFormat::Float32x4);

    /// Per vertex joint transform matrix index. Use in conjunction with [`Mesh::insert_attribute`]
    /// or [`Mesh::with_inserted_attribute`].
    ///
    /// The format of this attribute is [`VertexFormat::Uint16x4`].
    pub const ATTRIBUTE_JOINT_INDEX: MeshVertexAttribute =
        MeshVertexAttribute::new("Vertex_JointIndex", 7, VertexFormat::Uint16x4);

    /// The first index that can be used for custom vertex attributes.
    /// Only the attributes with an index below this are used by Bevy.
    pub const FIRST_AVAILABLE_CUSTOM_ATTRIBUTE: u64 = 8;

    /// Construct a new mesh. You need to provide a [`PrimitiveTopology`] so that the
    /// renderer knows how to treat the vertex data. Most of the time this will be
    /// [`PrimitiveTopology::TriangleList`].
    pub fn new(primitive_topology: PrimitiveTopology, asset_usage: RenderAssetUsages) -> Self {
        Mesh {
            primitive_topology,
            attributes: MeshExtractableData::Data(Default::default()),
            indices: MeshExtractableData::NoData,
            #[cfg(feature = "morph")]
            morph_targets: MeshExtractableData::NoData,
            #[cfg(feature = "morph")]
            morph_target_names: MeshExtractableData::NoData,
            asset_usage,
            enable_raytracing: true,
            final_aabb: None,
            skinned_mesh_bounds: None,
        }
    }

    /// Returns the topology of the mesh.
    pub fn primitive_topology(&self) -> PrimitiveTopology {
        self.primitive_topology
    }

    /// Sets the data for a vertex attribute (position, normal, etc.). The name will
    /// often be one of the associated constants such as [`Mesh::ATTRIBUTE_POSITION`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// # Panics
    /// Panics when the format of the values does not match the attribute's format.
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_insert_attribute`]
    #[inline]
    pub fn insert_attribute(
        &mut self,
        attribute: MeshVertexAttribute,
        values: impl Into<VertexAttributeValues>,
    ) {
        self.try_insert_attribute(attribute, values)
            .expect(MESH_EXTRACTED_ERROR);
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
    pub fn try_insert_attribute(
        &mut self,
        attribute: MeshVertexAttribute,
        values: impl Into<VertexAttributeValues>,
    ) -> Result<(), MeshAccessError> {
        let values = values.into();
        let values_format = VertexFormat::from(&values);
        if values_format != attribute.format {
            panic!(
                "Failed to insert attribute. Invalid attribute format for {}. Given format is {values_format:?} but expected {:?}",
                attribute.name, attribute.format
            );
        }

        self.attributes
            .as_mut()?
            .insert(attribute.id, MeshAttributeData { attribute, values });
        Ok(())
    }

    /// Consumes the mesh and returns a mesh with data set for a vertex attribute (position, normal, etc.).
    /// The name will often be one of the associated constants such as [`Mesh::ATTRIBUTE_POSITION`].
    ///
    /// (Alternatively, you can use [`Mesh::insert_attribute`] to mutate an existing mesh in-place)
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// # Panics
    /// Panics when the format of the values does not match the attribute's format.
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_with_inserted_attribute`]
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

    /// Consumes the mesh and returns a mesh with data set for a vertex attribute (position, normal, etc.).
    /// The name will often be one of the associated constants such as [`Mesh::ATTRIBUTE_POSITION`].
    ///
    /// (Alternatively, you can use [`Mesh::insert_attribute`] to mutate an existing mesh in-place)
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn try_with_inserted_attribute(
        mut self,
        attribute: MeshVertexAttribute,
        values: impl Into<VertexAttributeValues>,
    ) -> Result<Self, MeshAccessError> {
        self.try_insert_attribute(attribute, values)?;
        Ok(self)
    }

    /// Removes the data for a vertex attribute
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_remove_attribute`]
    pub fn remove_attribute(
        &mut self,
        attribute: impl Into<MeshVertexAttributeId>,
    ) -> Option<VertexAttributeValues> {
        self.attributes
            .as_mut()
            .expect(MESH_EXTRACTED_ERROR)
            .remove(&attribute.into())
            .map(|data| data.values)
    }

    /// Removes the data for a vertex attribute
    /// Returns an error if the mesh data has been extracted to `RenderWorld`or
    /// if the attribute does not exist.
    pub fn try_remove_attribute(
        &mut self,
        attribute: impl Into<MeshVertexAttributeId>,
    ) -> Result<VertexAttributeValues, MeshAccessError> {
        Ok(self
            .attributes
            .as_mut()?
            .remove(&attribute.into())
            .ok_or(MeshAccessError::NotFound)?
            .values)
    }

    /// Consumes the mesh and returns a mesh without the data for a vertex attribute
    ///
    /// (Alternatively, you can use [`Mesh::remove_attribute`] to mutate an existing mesh in-place)
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_with_removed_attribute`]
    #[must_use]
    pub fn with_removed_attribute(mut self, attribute: impl Into<MeshVertexAttributeId>) -> Self {
        self.remove_attribute(attribute);
        self
    }

    /// Consumes the mesh and returns a mesh without the data for a vertex attribute
    ///
    /// (Alternatively, you can use [`Mesh::remove_attribute`] to mutate an existing mesh in-place)
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`or
    /// if the attribute does not exist.
    pub fn try_with_removed_attribute(
        mut self,
        attribute: impl Into<MeshVertexAttributeId>,
    ) -> Result<Self, MeshAccessError> {
        self.try_remove_attribute(attribute)?;
        Ok(self)
    }

    /// Returns a bool indicating if the attribute is present in this mesh's vertex data.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_contains_attribute`]
    #[inline]
    pub fn contains_attribute(&self, id: impl Into<MeshVertexAttributeId>) -> bool {
        self.attributes
            .as_ref()
            .expect(MESH_EXTRACTED_ERROR)
            .contains_key(&id.into())
    }

    /// Returns a bool indicating if the attribute is present in this mesh's vertex data.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn try_contains_attribute(
        &self,
        id: impl Into<MeshVertexAttributeId>,
    ) -> Result<bool, MeshAccessError> {
        Ok(self.attributes.as_ref()?.contains_key(&id.into()))
    }

    /// Retrieves the data currently set to the vertex attribute with the specified [`MeshVertexAttributeId`].
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_attribute`] or [`Mesh::try_attribute_option`]
    #[inline]
    pub fn attribute(
        &self,
        id: impl Into<MeshVertexAttributeId>,
    ) -> Option<&VertexAttributeValues> {
        self.try_attribute_option(id).expect(MESH_EXTRACTED_ERROR)
    }

    /// Retrieves the data currently set to the vertex attribute with the specified [`MeshVertexAttributeId`].
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`or
    /// if the attribute does not exist.
    #[inline]
    pub fn try_attribute(
        &self,
        id: impl Into<MeshVertexAttributeId>,
    ) -> Result<&VertexAttributeValues, MeshAccessError> {
        self.try_attribute_option(id)?
            .ok_or(MeshAccessError::NotFound)
    }

    /// Retrieves the data currently set to the vertex attribute with the specified [`MeshVertexAttributeId`].
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn try_attribute_option(
        &self,
        id: impl Into<MeshVertexAttributeId>,
    ) -> Result<Option<&VertexAttributeValues>, MeshAccessError> {
        Ok(self
            .attributes
            .as_ref()?
            .get(&id.into())
            .map(|data| &data.values))
    }

    /// Retrieves the full data currently set to the vertex attribute with the specified [`MeshVertexAttributeId`].
    #[inline]
    pub(crate) fn try_attribute_data(
        &self,
        id: impl Into<MeshVertexAttributeId>,
    ) -> Result<Option<&MeshAttributeData>, MeshAccessError> {
        Ok(self.attributes.as_ref()?.get(&id.into()))
    }

    /// Retrieves the data currently set to the vertex attribute with the specified `name` mutably.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_attribute_mut`]
    #[inline]
    pub fn attribute_mut(
        &mut self,
        id: impl Into<MeshVertexAttributeId>,
    ) -> Option<&mut VertexAttributeValues> {
        self.try_attribute_mut_option(id)
            .expect(MESH_EXTRACTED_ERROR)
    }

    /// Retrieves the data currently set to the vertex attribute with the specified `name` mutably.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`or
    /// if the attribute does not exist.
    #[inline]
    pub fn try_attribute_mut(
        &mut self,
        id: impl Into<MeshVertexAttributeId>,
    ) -> Result<&mut VertexAttributeValues, MeshAccessError> {
        self.try_attribute_mut_option(id)?
            .ok_or(MeshAccessError::NotFound)
    }

    /// Retrieves the data currently set to the vertex attribute with the specified `name` mutably.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn try_attribute_mut_option(
        &mut self,
        id: impl Into<MeshVertexAttributeId>,
    ) -> Result<Option<&mut VertexAttributeValues>, MeshAccessError> {
        Ok(self
            .attributes
            .as_mut()?
            .get_mut(&id.into())
            .map(|data| &mut data.values))
    }

    /// Returns an iterator that yields references to the data of each vertex attribute.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_attributes`]
    pub fn attributes(
        &self,
    ) -> impl Iterator<Item = (&MeshVertexAttribute, &VertexAttributeValues)> {
        self.try_attributes().expect(MESH_EXTRACTED_ERROR)
    }

    /// Returns an iterator that yields references to the data of each vertex attribute.
    /// Returns an error if data has been extracted to `RenderWorld`
    pub fn try_attributes(
        &self,
    ) -> Result<impl Iterator<Item = (&MeshVertexAttribute, &VertexAttributeValues)>, MeshAccessError>
    {
        Ok(self
            .attributes
            .as_ref()?
            .values()
            .map(|data| (&data.attribute, &data.values)))
    }

    /// Returns an iterator that yields mutable references to the data of each vertex attribute.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_attributes_mut`]
    pub fn attributes_mut(
        &mut self,
    ) -> impl Iterator<Item = (&MeshVertexAttribute, &mut VertexAttributeValues)> {
        self.try_attributes_mut().expect(MESH_EXTRACTED_ERROR)
    }

    /// Returns an iterator that yields mutable references to the data of each vertex attribute.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    pub fn try_attributes_mut(
        &mut self,
    ) -> Result<
        impl Iterator<Item = (&MeshVertexAttribute, &mut VertexAttributeValues)>,
        MeshAccessError,
    > {
        Ok(self
            .attributes
            .as_mut()?
            .values_mut()
            .map(|data| (&data.attribute, &mut data.values)))
    }

    /// Sets the vertex indices of the mesh. They describe how triangles are constructed out of the
    /// vertex attributes and are therefore only useful for the [`PrimitiveTopology`] variants
    /// that use triangles.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_insert_indices`]
    #[inline]
    pub fn insert_indices(&mut self, indices: Indices) {
        self.indices
            .replace(Some(indices))
            .expect(MESH_EXTRACTED_ERROR);
    }

    /// Sets the vertex indices of the mesh. They describe how triangles are constructed out of the
    /// vertex attributes and are therefore only useful for the [`PrimitiveTopology`] variants
    /// that use triangles.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn try_insert_indices(&mut self, indices: Indices) -> Result<(), MeshAccessError> {
        self.indices.replace(Some(indices))?;
        Ok(())
    }

    /// Consumes the mesh and returns a mesh with the given vertex indices. They describe how triangles
    /// are constructed out of the vertex attributes and are therefore only useful for the
    /// [`PrimitiveTopology`] variants that use triangles.
    ///
    /// (Alternatively, you can use [`Mesh::insert_indices`] to mutate an existing mesh in-place)
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_with_inserted_indices`]
    #[must_use]
    #[inline]
    pub fn with_inserted_indices(mut self, indices: Indices) -> Self {
        self.insert_indices(indices);
        self
    }

    /// Consumes the mesh and returns a mesh with the given vertex indices. They describe how triangles
    /// are constructed out of the vertex attributes and are therefore only useful for the
    /// [`PrimitiveTopology`] variants that use triangles.
    ///
    /// (Alternatively, you can use [`Mesh::try_insert_indices`] to mutate an existing mesh in-place)
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn try_with_inserted_indices(mut self, indices: Indices) -> Result<Self, MeshAccessError> {
        self.try_insert_indices(indices)?;
        Ok(self)
    }

    /// Retrieves the vertex `indices` of the mesh, returns None if not found.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_indices`]
    #[inline]
    pub fn indices(&self) -> Option<&Indices> {
        self.indices.as_ref_option().expect(MESH_EXTRACTED_ERROR)
    }

    /// Retrieves the vertex `indices` of the mesh.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`or
    /// if the attribute does not exist.
    #[inline]
    pub fn try_indices(&self) -> Result<&Indices, MeshAccessError> {
        self.indices.as_ref()
    }

    /// Retrieves the vertex `indices` of the mesh, returns None if not found.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn try_indices_option(&self) -> Result<Option<&Indices>, MeshAccessError> {
        self.indices.as_ref_option()
    }

    /// Retrieves the vertex `indices` of the mesh mutably.
    #[inline]
    pub fn indices_mut(&mut self) -> Option<&mut Indices> {
        self.try_indices_mut_option().expect(MESH_EXTRACTED_ERROR)
    }

    /// Retrieves the vertex `indices` of the mesh mutably.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn try_indices_mut(&mut self) -> Result<&mut Indices, MeshAccessError> {
        self.indices.as_mut()
    }

    /// Retrieves the vertex `indices` of the mesh mutably.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn try_indices_mut_option(&mut self) -> Result<Option<&mut Indices>, MeshAccessError> {
        self.indices.as_mut_option()
    }

    /// Removes the vertex `indices` from the mesh and returns them.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_remove_indices`]
    #[inline]
    pub fn remove_indices(&mut self) -> Option<Indices> {
        self.try_remove_indices().expect(MESH_EXTRACTED_ERROR)
    }

    /// Removes the vertex `indices` from the mesh and returns them.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    #[inline]
    pub fn try_remove_indices(&mut self) -> Result<Option<Indices>, MeshAccessError> {
        self.indices.replace(None)
    }

    /// Consumes the mesh and returns a mesh without the vertex `indices` of the mesh.
    ///
    /// (Alternatively, you can use [`Mesh::remove_indices`] to mutate an existing mesh in-place)
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_with_removed_indices`]
    #[must_use]
    pub fn with_removed_indices(mut self) -> Self {
        self.remove_indices();
        self
    }

    /// Consumes the mesh and returns a mesh without the vertex `indices` of the mesh.
    ///
    /// (Alternatively, you can use [`Mesh::try_remove_indices`] to mutate an existing mesh in-place)
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    pub fn try_with_removed_indices(mut self) -> Result<Self, MeshAccessError> {
        self.try_remove_indices()?;
        Ok(self)
    }

    /// Returns the size of a vertex in bytes.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`.
    pub fn get_vertex_size(&self) -> u64 {
        self.attributes
            .as_ref()
            .expect(MESH_EXTRACTED_ERROR)
            .values()
            .map(|data| data.attribute.format.size())
            .sum()
    }

    /// Returns the size required for the vertex buffer in bytes.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`.
    pub fn get_vertex_buffer_size(&self) -> usize {
        let vertex_size = self.get_vertex_size() as usize;
        let vertex_count = self.count_vertices();
        vertex_count * vertex_size
    }

    /// Computes and returns the index data of the mesh as bytes.
    /// This is used to transform the index data into a GPU friendly format.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`.
    pub fn get_index_buffer_bytes(&self) -> Option<&[u8]> {
        let mesh_indices = self.indices.as_ref_option().expect(MESH_EXTRACTED_ERROR);

        mesh_indices.as_ref().map(|indices| match &indices {
            Indices::U16(indices) => cast_slice(&indices[..]),
            Indices::U32(indices) => cast_slice(&indices[..]),
        })
    }

    /// Get this `Mesh`'s [`MeshVertexBufferLayout`], used in `SpecializedMeshPipeline`.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`.
    pub fn get_mesh_vertex_buffer_layout(
        &self,
        mesh_vertex_buffer_layouts: &mut MeshVertexBufferLayouts,
    ) -> MeshVertexBufferLayoutRef {
        let mesh_attributes = self.attributes.as_ref().expect(MESH_EXTRACTED_ERROR);

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
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`.
    pub fn count_vertices(&self) -> usize {
        let mut vertex_count: Option<usize> = None;
        let mesh_attributes = self.attributes.as_ref().expect(MESH_EXTRACTED_ERROR);

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
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`.
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
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`.
    pub fn write_packed_vertex_buffer_data(&self, slice: &mut [u8]) {
        let mesh_attributes = self.attributes.as_ref().expect(MESH_EXTRACTED_ERROR);

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
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_duplicate_vertices`]
    pub fn duplicate_vertices(&mut self) {
        self.try_duplicate_vertices().expect(MESH_EXTRACTED_ERROR);
    }

    /// Duplicates the vertex attributes so that no vertices are shared.
    ///
    /// This can dramatically increase the vertex count, so make sure this is what you want.
    /// Does nothing if no [Indices] are set.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    pub fn try_duplicate_vertices(&mut self) -> Result<(), MeshAccessError> {
        fn duplicate<T: Copy>(values: &[T], indices: impl Iterator<Item = usize>) -> Vec<T> {
            indices.map(|i| values[i]).collect()
        }

        let Some(indices) = self.indices.replace(None)? else {
            return Ok(());
        };

        let mesh_attributes = self.attributes.as_mut()?;

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

        Ok(())
    }

    /// Consumes the mesh and returns a mesh with no shared vertices.
    ///
    /// This can dramatically increase the vertex count, so make sure this is what you want.
    /// Does nothing if no [`Indices`] are set.
    ///
    /// (Alternatively, you can use [`Mesh::duplicate_vertices`] to mutate an existing mesh in-place)
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_with_duplicated_vertices`]
    #[must_use]
    pub fn with_duplicated_vertices(mut self) -> Self {
        self.duplicate_vertices();
        self
    }

    /// Consumes the mesh and returns a mesh with no shared vertices.
    ///
    /// This can dramatically increase the vertex count, so make sure this is what you want.
    /// Does nothing if no [`Indices`] are set.
    ///
    /// (Alternatively, you can use [`Mesh::try_duplicate_vertices`] to mutate an existing mesh in-place)
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    pub fn try_with_duplicated_vertices(mut self) -> Result<Self, MeshAccessError> {
        self.try_duplicate_vertices()?;
        Ok(self)
    }

    /// Remove duplicate vertices and create the index pointing to the unique vertices.
    ///
    /// This function is no-op if the mesh already has [`Indices`] set,
    /// even if there are duplicate vertices. If deduplication is needed with indices already set,
    /// consider calling [`Mesh::duplicate_vertices`] and then this function.
    pub fn deduplicate_vertices(&mut self) {
        if self.indices.is_some() {
            return;
        }

        #[derive(Copy, Clone)]
        struct VertexRef<'a> {
            mesh: &'a Mesh,
            i: usize,
        }
        impl<'a> VertexRef<'a> {
            fn push_to(&self, target: &mut BTreeMap<MeshVertexAttributeId, MeshAttributeData>) {
                for (key, this_attribute_data) in self.mesh.attributes.iter() {
                    let target_attribute_data = target.get_mut(key).unwrap();
                    target_attribute_data
                        .values
                        .push_from(&this_attribute_data.values, self.i);
                }
            }
        }
        impl<'a> PartialEq for VertexRef<'a> {
            fn eq(&self, other: &Self) -> bool {
                assert!(ptr::eq(self.mesh, other.mesh));
                for values in self.mesh.attributes.values() {
                    if values.values.get_bytes_at(self.i) != values.values.get_bytes_at(other.i) {
                        return false;
                    }
                }
                true
            }
        }
        impl<'a> Eq for VertexRef<'a> {}
        impl<'a> Hash for VertexRef<'a> {
            fn hash<H: Hasher>(&self, state: &mut H) {
                for values in self.mesh.attributes.values() {
                    values.values.get_bytes_at(self.i).hash(state);
                }
            }
        }

        let mut new_attributes: BTreeMap<MeshVertexAttributeId, MeshAttributeData> = self
            .attributes
            .iter()
            .map(|(k, v)| {
                (
                    *k,
                    MeshAttributeData {
                        attribute: v.attribute,
                        values: VertexAttributeValues::new(VertexFormat::from(&v.values)),
                    },
                )
            })
            .collect();

        let mut vertex_to_new_index: HashMap<VertexRef, u32> = HashMap::new();
        let mut indices = Vec::with_capacity(self.count_vertices());
        for i in 0..self.count_vertices() {
            let len: u32 = vertex_to_new_index
                .len()
                .try_into()
                .expect("The number of vertices exceeds u32::MAX");
            let vertex_ref = VertexRef { mesh: self, i };
            let j = match vertex_to_new_index.entry(vertex_ref) {
                hash_map::Entry::Occupied(e) => *e.get(),
                hash_map::Entry::Vacant(e) => {
                    e.insert(len);
                    vertex_ref.push_to(&mut new_attributes);
                    len
                }
            };
            indices.push(j);
        }
        drop(vertex_to_new_index);

        for v in new_attributes.values_mut() {
            v.values.shrink_to_fit();
        }

        self.attributes = new_attributes;
        self.indices = Some(Indices::U32(indices));
    }

    /// Consumes the mesh and returns a mesh with merged vertices.
    ///
    /// See [`Mesh::deduplicate_vertices`] for more information.
    #[must_use]
    pub fn with_deduplicated_vertices(mut self) -> Self {
        self.deduplicate_vertices();
        self
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

        let mesh_indices = self.indices.as_mut_option()?;

        match mesh_indices {
            Some(Indices::U16(vec)) => invert(vec, self.primitive_topology),
            Some(Indices::U32(vec)) => invert(vec, self.primitive_topology),
            None => Ok(()),
        }
    }

    /// Consumes the mesh and returns a mesh with inverted winding of the indices such
    /// that all counter-clockwise triangles are now clockwise and vice versa.
    ///
    /// Does nothing if no [`Indices`] are set.
    pub fn with_inverted_winding(mut self) -> Result<Self, MeshWindingInvertError> {
        self.invert_winding().map(|_| self)
    }

    /// Calculates the [`Mesh::ATTRIBUTE_NORMAL`] of a mesh.
    /// If the mesh is indexed, this defaults to smooth normals. Otherwise, it defaults to flat
    /// normals.
    ///
    /// # Panics
    /// Panics if [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].=
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_compute_normals`]
    pub fn compute_normals(&mut self) {
        self.try_compute_normals().expect(MESH_EXTRACTED_ERROR);
    }

    /// Calculates the [`Mesh::ATTRIBUTE_NORMAL`] of a mesh.
    /// If the mesh is indexed, this defaults to smooth normals. Otherwise, it defaults to flat
    /// normals.
    ///
    /// # Panics
    /// Panics if [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].=
    pub fn try_compute_normals(&mut self) -> Result<(), MeshAccessError> {
        assert!(
            matches!(self.primitive_topology, PrimitiveTopology::TriangleList),
            "`compute_normals` can only work on `TriangleList`s"
        );
        if self.try_indices_option()?.is_none() {
            self.try_compute_flat_normals()
        } else {
            self.try_compute_smooth_normals()
        }
    }

    /// Calculates the [`Mesh::ATTRIBUTE_NORMAL`] of a mesh.
    ///
    /// # Panics
    /// Panics if [`Indices`] are set or [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].
    /// Consider calling [`Mesh::duplicate_vertices`] or exporting your mesh with normal
    /// attributes.
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_compute_flat_normals`]
    ///
    /// FIXME: This should handle more cases since this is called as a part of gltf
    /// mesh loading where we can't really blame users for loading meshes that might
    /// not conform to the limitations here!
    pub fn compute_flat_normals(&mut self) {
        self.try_compute_flat_normals().expect(MESH_EXTRACTED_ERROR);
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
    pub fn try_compute_flat_normals(&mut self) -> Result<(), MeshAccessError> {
        assert!(
            self.try_indices_option()?.is_none(),
            "`compute_flat_normals` can't work on indexed geometry. Consider calling either `Mesh::compute_smooth_normals` or `Mesh::duplicate_vertices` followed by `Mesh::compute_flat_normals`."
        );
        assert!(
            matches!(self.primitive_topology, PrimitiveTopology::TriangleList),
            "`compute_flat_normals` can only work on `TriangleList`s"
        );

        let positions = self
            .try_attribute(Mesh::ATTRIBUTE_POSITION)?
            .as_float3()
            .expect("`Mesh::ATTRIBUTE_POSITION` vertex attributes should be of type `float3`");

        let normals: Vec<_> = positions
            .chunks_exact(3)
            .map(|p| triangle_normal(p[0], p[1], p[2]))
            .flat_map(|normal| [normal; 3])
            .collect();

        self.try_insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
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
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_compute_smooth_normals`]
    pub fn compute_smooth_normals(&mut self) {
        self.try_compute_smooth_normals()
            .expect(MESH_EXTRACTED_ERROR);
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
    pub fn try_compute_smooth_normals(&mut self) -> Result<(), MeshAccessError> {
        self.try_compute_custom_smooth_normals(|[a, b, c], positions, normals| {
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
        })
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
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_compute_area_weighted_normals`]
    pub fn compute_area_weighted_normals(&mut self) {
        self.try_compute_area_weighted_normals()
            .expect(MESH_EXTRACTED_ERROR);
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
    pub fn try_compute_area_weighted_normals(&mut self) -> Result<(), MeshAccessError> {
        self.try_compute_custom_smooth_normals(|[a, b, c], positions, normals| {
            let normal = Vec3::from(triangle_area_normal(
                positions[a],
                positions[b],
                positions[c],
            ));
            [a, b, c].into_iter().for_each(|pos| {
                normals[pos] += normal;
            });
        })
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
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_compute_custom_smooth_normals`]
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
        per_triangle: impl FnMut([usize; 3], &[[f32; 3]], &mut [Vec3]),
    ) {
        self.try_compute_custom_smooth_normals(per_triangle)
            .expect(MESH_EXTRACTED_ERROR);
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
    pub fn try_compute_custom_smooth_normals(
        &mut self,
        mut per_triangle: impl FnMut([usize; 3], &[[f32; 3]], &mut [Vec3]),
    ) -> Result<(), MeshAccessError> {
        assert!(
            matches!(self.primitive_topology, PrimitiveTopology::TriangleList),
            "smooth normals can only be computed on `TriangleList`s"
        );
        assert!(
            self.try_indices_option()?.is_some(),
            "smooth normals can only be computed on indexed meshes"
        );

        let positions = self
            .try_attribute(Mesh::ATTRIBUTE_POSITION)?
            .as_float3()
            .expect("`Mesh::ATTRIBUTE_POSITION` vertex attributes should be of type `float3`");

        let mut normals = vec![Vec3::ZERO; positions.len()];

        self.try_indices()?
            .iter()
            .collect::<Vec<usize>>()
            .chunks_exact(3)
            .for_each(|face| per_triangle([face[0], face[1], face[2]], positions, &mut normals));

        for normal in &mut normals {
            *normal = normal.try_normalize().unwrap_or(Vec3::ZERO);
        }

        self.try_insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
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
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_with_computed_normals`]
    #[must_use]
    pub fn with_computed_normals(self) -> Self {
        self.try_with_computed_normals()
            .expect(MESH_EXTRACTED_ERROR)
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
    pub fn try_with_computed_normals(mut self) -> Result<Self, MeshAccessError> {
        self.try_compute_normals()?;
        Ok(self)
    }

    /// Consumes the mesh and returns a mesh with calculated [`Mesh::ATTRIBUTE_NORMAL`].
    ///
    /// (Alternatively, you can use [`Mesh::compute_flat_normals`] to mutate an existing mesh in-place)
    ///
    /// # Panics
    /// Panics if [`Mesh::ATTRIBUTE_POSITION`] is not of type `float3`.
    /// Panics if the mesh has any other topology than [`PrimitiveTopology::TriangleList`].
    /// Panics if the mesh has indices defined
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_with_computed_flat_normals`]
    pub fn with_computed_flat_normals(mut self) -> Self {
        self.compute_flat_normals();
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
    pub fn try_with_computed_flat_normals(mut self) -> Result<Self, MeshAccessError> {
        self.try_compute_flat_normals()?;
        Ok(self)
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
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_with_computed_smooth_normals`]
    pub fn with_computed_smooth_normals(mut self) -> Self {
        self.compute_smooth_normals();
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
    pub fn try_with_computed_smooth_normals(mut self) -> Result<Self, MeshAccessError> {
        self.try_compute_smooth_normals()?;
        Ok(self)
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
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_with_computed_area_weighted_normals`]
    pub fn with_computed_area_weighted_normals(mut self) -> Self {
        self.compute_area_weighted_normals();
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
    pub fn try_with_computed_area_weighted_normals(mut self) -> Result<Self, MeshAccessError> {
        self.try_compute_area_weighted_normals()?;
        Ok(self)
    }

    /// Generate tangents for the mesh using the `mikktspace` algorithm.
    ///
    /// Sets the [`Mesh::ATTRIBUTE_TANGENT`] attribute if successful.
    /// Requires a [`PrimitiveTopology::TriangleList`] topology and the [`Mesh::ATTRIBUTE_POSITION`], [`Mesh::ATTRIBUTE_NORMAL`] and [`Mesh::ATTRIBUTE_UV_0`] attributes set.
    #[cfg(feature = "bevy_mikktspace")]
    pub fn generate_tangents(&mut self) -> Result<(), super::GenerateTangentsError> {
        let tangents = super::generate_tangents_for_mesh(self)?;
        self.try_insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents)?;
        Ok(())
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
    /// * Both meshes must have the same primitive topology.
    pub fn merge(&mut self, other: &Mesh) -> Result<(), MeshMergeError> {
        use VertexAttributeValues::*;

        // Check if the meshes `primitive_topology` field is the same,
        // as if that is not the case, the resulting mesh could (and most likely would)
        // be invalid.
        if self.primitive_topology != other.primitive_topology {
            return Err(MeshMergeError::IncompatiblePrimitiveTopology {
                self_primitive_topology: self.primitive_topology,
                other_primitive_topology: other.primitive_topology,
            });
        }

        // The indices of `other` should start after the last vertex of `self`.
        let index_offset = self.count_vertices();

        // Extend attributes of `self` with attributes of `other`.
        for (attribute, values) in self.try_attributes_mut()? {
            if let Some(other_values) = other.try_attribute_option(attribute.id)? {
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
                                .try_attribute_data(attribute.id)?
                                .map(|data| data.attribute),
                        })
                    }
                }
            }
        }

        // Extend indices of `self` with indices of `other`.
        if let (Some(indices), Some(other_indices)) =
            (self.try_indices_mut_option()?, other.try_indices_option()?)
        {
            indices.extend(other_indices.iter().map(|i| (i + index_offset) as u32));
        }
        Ok(())
    }

    /// Transforms the vertex positions, normals, and tangents of the mesh by the given [`Transform`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_transformed_by`]
    pub fn transformed_by(mut self, transform: Transform) -> Self {
        self.transform_by(transform);
        self
    }

    /// Transforms the vertex positions, normals, and tangents of the mesh by the given [`Transform`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    pub fn try_transformed_by(mut self, transform: Transform) -> Result<Self, MeshAccessError> {
        self.try_transform_by(transform)?;
        Ok(self)
    }

    /// Transforms the vertex positions, normals, and tangents of the mesh in place by the given [`Transform`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_transform_by`]
    pub fn transform_by(&mut self, transform: Transform) {
        self.try_transform_by(transform)
            .expect(MESH_EXTRACTED_ERROR);
    }

    /// Transforms the vertex positions, normals, and tangents of the mesh in place by the given [`Transform`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    pub fn try_transform_by(&mut self, transform: Transform) -> Result<(), MeshAccessError> {
        // Needed when transforming normals and tangents
        let scale_recip = 1. / transform.scale;
        debug_assert!(
            transform.scale.yzx() * transform.scale.zxy() != Vec3::ZERO,
            "mesh transform scale cannot be zero on more than one axis"
        );

        if let Some(VertexAttributeValues::Float32x3(positions)) =
            self.try_attribute_mut_option(Mesh::ATTRIBUTE_POSITION)?
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
            return Ok(());
        }

        if let Some(VertexAttributeValues::Float32x3(normals)) =
            self.try_attribute_mut_option(Mesh::ATTRIBUTE_NORMAL)?
        {
            // Transform normals, taking into account non-uniform scaling and rotation
            normals.iter_mut().for_each(|normal| {
                *normal = (transform.rotation
                    * scale_normal(Vec3::from_array(*normal), scale_recip))
                .to_array();
            });
        }

        if let Some(VertexAttributeValues::Float32x4(tangents)) =
            self.try_attribute_mut_option(Mesh::ATTRIBUTE_TANGENT)?
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

        Ok(())
    }

    /// Translates the vertex positions of the mesh by the given [`Vec3`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_translated_by`]
    pub fn translated_by(mut self, translation: Vec3) -> Self {
        self.translate_by(translation);
        self
    }

    /// Translates the vertex positions of the mesh by the given [`Vec3`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    pub fn try_translated_by(mut self, translation: Vec3) -> Result<Self, MeshAccessError> {
        self.try_translate_by(translation)?;
        Ok(self)
    }

    /// Translates the vertex positions of the mesh in place by the given [`Vec3`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_translate_by`]
    pub fn translate_by(&mut self, translation: Vec3) {
        self.try_translate_by(translation)
            .expect(MESH_EXTRACTED_ERROR);
    }

    /// Translates the vertex positions of the mesh in place by the given [`Vec3`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    pub fn try_translate_by(&mut self, translation: Vec3) -> Result<(), MeshAccessError> {
        if translation == Vec3::ZERO {
            return Ok(());
        }

        if let Some(VertexAttributeValues::Float32x3(positions)) =
            self.try_attribute_mut_option(Mesh::ATTRIBUTE_POSITION)?
        {
            // Apply translation to vertex positions
            positions
                .iter_mut()
                .for_each(|pos| *pos = (Vec3::from_slice(pos) + translation).to_array());
        }

        Ok(())
    }

    /// Rotates the vertex positions, normals, and tangents of the mesh by the given [`Quat`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_rotated_by`]
    pub fn rotated_by(mut self, rotation: Quat) -> Self {
        self.try_rotate_by(rotation).expect(MESH_EXTRACTED_ERROR);
        self
    }

    /// Rotates the vertex positions, normals, and tangents of the mesh by the given [`Quat`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    pub fn try_rotated_by(mut self, rotation: Quat) -> Result<Self, MeshAccessError> {
        self.try_rotate_by(rotation)?;
        Ok(self)
    }

    /// Rotates the vertex positions, normals, and tangents of the mesh in place by the given [`Quat`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_rotate_by`]
    pub fn rotate_by(&mut self, rotation: Quat) {
        self.try_rotate_by(rotation).expect(MESH_EXTRACTED_ERROR);
    }

    /// Rotates the vertex positions, normals, and tangents of the mesh in place by the given [`Quat`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    pub fn try_rotate_by(&mut self, rotation: Quat) -> Result<(), MeshAccessError> {
        if let Some(VertexAttributeValues::Float32x3(positions)) =
            self.try_attribute_mut_option(Mesh::ATTRIBUTE_POSITION)?
        {
            // Apply rotation to vertex positions
            positions
                .iter_mut()
                .for_each(|pos| *pos = (rotation * Vec3::from_slice(pos)).to_array());
        }

        // No need to transform normals or tangents if rotation is near identity
        if rotation.is_near_identity() {
            return Ok(());
        }

        if let Some(VertexAttributeValues::Float32x3(normals)) =
            self.try_attribute_mut_option(Mesh::ATTRIBUTE_NORMAL)?
        {
            // Transform normals
            normals.iter_mut().for_each(|normal| {
                *normal = (rotation * Vec3::from_slice(normal).normalize_or_zero()).to_array();
            });
        }

        if let Some(VertexAttributeValues::Float32x4(tangents)) =
            self.try_attribute_mut_option(Mesh::ATTRIBUTE_TANGENT)?
        {
            // Transform tangents
            tangents.iter_mut().for_each(|tangent| {
                let handedness = tangent[3];
                *tangent = (rotation * Vec3::from_slice(tangent).normalize_or_zero())
                    .extend(handedness)
                    .to_array();
            });
        }

        Ok(())
    }

    /// Scales the vertex positions, normals, and tangents of the mesh by the given [`Vec3`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_scaled_by`]
    pub fn scaled_by(mut self, scale: Vec3) -> Self {
        self.scale_by(scale);
        self
    }

    /// Scales the vertex positions, normals, and tangents of the mesh by the given [`Vec3`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    pub fn try_scaled_by(mut self, scale: Vec3) -> Result<Self, MeshAccessError> {
        self.try_scale_by(scale)?;
        Ok(self)
    }

    /// Scales the vertex positions, normals, and tangents of the mesh in place by the given [`Vec3`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_scale_by`]
    pub fn scale_by(&mut self, scale: Vec3) {
        self.try_scale_by(scale).expect(MESH_EXTRACTED_ERROR);
    }

    /// Scales the vertex positions, normals, and tangents of the mesh in place by the given [`Vec3`].
    ///
    /// `Aabb` of entities with modified mesh are not updated automatically.
    pub fn try_scale_by(&mut self, scale: Vec3) -> Result<(), MeshAccessError> {
        // Needed when transforming normals and tangents
        let scale_recip = 1. / scale;
        debug_assert!(
            scale.yzx() * scale.zxy() != Vec3::ZERO,
            "mesh transform scale cannot be zero on more than one axis"
        );

        if let Some(VertexAttributeValues::Float32x3(positions)) =
            self.try_attribute_mut_option(Mesh::ATTRIBUTE_POSITION)?
        {
            // Apply scale to vertex positions
            positions
                .iter_mut()
                .for_each(|pos| *pos = (scale * Vec3::from_slice(pos)).to_array());
        }

        // No need to transform normals or tangents if scale is uniform
        if scale.x == scale.y && scale.y == scale.z {
            return Ok(());
        }

        if let Some(VertexAttributeValues::Float32x3(normals)) =
            self.try_attribute_mut_option(Mesh::ATTRIBUTE_NORMAL)?
        {
            // Transform normals, taking into account non-uniform scaling
            normals.iter_mut().for_each(|normal| {
                *normal = scale_normal(Vec3::from_array(*normal), scale_recip).to_array();
            });
        }

        if let Some(VertexAttributeValues::Float32x4(tangents)) =
            self.try_attribute_mut_option(Mesh::ATTRIBUTE_TANGENT)?
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

        Ok(())
    }

    /// Normalize joint weights so they sum to 1.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_normalize_joint_weights`]
    pub fn normalize_joint_weights(&mut self) {
        self.try_normalize_joint_weights()
            .expect(MESH_EXTRACTED_ERROR);
    }

    /// Normalize joint weights so they sum to 1.
    pub fn try_normalize_joint_weights(&mut self) -> Result<(), MeshAccessError> {
        if let Some(VertexAttributeValues::Float32x4(joints)) =
            self.try_attribute_mut_option(Self::ATTRIBUTE_JOINT_WEIGHT)?
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
    pub fn triangles(&self) -> Result<impl Iterator<Item = Triangle3d> + '_, MeshTrianglesError> {
        let position_data = self.try_attribute(Mesh::ATTRIBUTE_POSITION)?;

        let Some(vertices) = position_data.as_float3() else {
            return Err(MeshTrianglesError::PositionsFormat);
        };

        let indices = self.try_indices()?;

        match self.primitive_topology {
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

    /// Extracts the mesh vertex, index and morph target data for GPU upload.
    /// This function is called internally in render world extraction, it is
    /// unlikely to be useful outside of that context.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    pub fn take_gpu_data(&mut self) -> Result<Self, MeshAccessError> {
        let attributes = self.attributes.extract()?;
        let indices = self.indices.extract()?;
        #[cfg(feature = "morph")]
        let morph_targets = self.morph_targets.extract()?;
        #[cfg(feature = "morph")]
        let morph_target_names = self.morph_target_names.extract()?;

        // store the aabb extents as they cannot be computed after extraction
        if let Some(MeshAttributeData {
            values: VertexAttributeValues::Float32x3(position_values),
            ..
        }) = attributes
            .as_ref_option()?
            .and_then(|attrs| attrs.get(&Self::ATTRIBUTE_POSITION.id))
            && !position_values.is_empty()
        {
            let mut iter = position_values.iter().map(|p| Vec3::from_slice(p));
            let mut min = iter.next().unwrap();
            let mut max = min;
            for v in iter {
                min = Vec3::min(min, v);
                max = Vec3::max(max, v);
            }
            self.final_aabb = Some(Aabb3d::from_min_max(min, max));
        }

        Ok(Self {
            attributes,
            indices,
            #[cfg(feature = "morph")]
            morph_targets,
            #[cfg(feature = "morph")]
            morph_target_names,
            ..self.clone()
        })
    }

    /// Get this mesh's [`SkinnedMeshBounds`].
    pub fn skinned_mesh_bounds(&self) -> Option<&SkinnedMeshBounds> {
        self.skinned_mesh_bounds.as_ref()
    }

    /// Set this mesh's [`SkinnedMeshBounds`].
    pub fn set_skinned_mesh_bounds(&mut self, skinned_mesh_bounds: Option<SkinnedMeshBounds>) {
        self.skinned_mesh_bounds = skinned_mesh_bounds;
    }

    /// Consumes the mesh and returns a mesh with the given [`SkinnedMeshBounds`].
    pub fn with_skinned_mesh_bounds(
        mut self,
        skinned_mesh_bounds: Option<SkinnedMeshBounds>,
    ) -> Self {
        self.set_skinned_mesh_bounds(skinned_mesh_bounds);
        self
    }

    /// Generate [`SkinnedMeshBounds`] for this mesh.
    pub fn generate_skinned_mesh_bounds(&mut self) -> Result<(), SkinnedMeshBoundsError> {
        self.skinned_mesh_bounds = Some(SkinnedMeshBounds::from_mesh(self)?);
        Ok(())
    }

    /// Consumes the mesh and returns a mesh with generated [`SkinnedMeshBounds`].
    pub fn with_generated_skinned_mesh_bounds(mut self) -> Result<Self, SkinnedMeshBoundsError> {
        self.generate_skinned_mesh_bounds()?;
        Ok(self)
    }
}

#[cfg(feature = "morph")]
impl Mesh {
    /// Whether this mesh has morph targets.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_has_morph_targets`]
    pub fn has_morph_targets(&self) -> bool {
        self.try_has_morph_targets().expect(MESH_EXTRACTED_ERROR)
    }

    /// Whether this mesh has morph targets.
    pub fn try_has_morph_targets(&self) -> Result<bool, MeshAccessError> {
        Ok(self.morph_targets.as_ref_option()?.is_some())
    }

    /// Set [morph targets] image for this mesh. This requires a "morph target image". See [`MorphTargetImage`](crate::morph::MorphTargetImage) for info.
    ///
    /// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_set_morph_targets`]
    pub fn set_morph_targets(&mut self, morph_targets: Handle<Image>) {
        self.try_set_morph_targets(morph_targets)
            .expect(MESH_EXTRACTED_ERROR);
    }

    /// Set [morph targets] image for this mesh. This requires a "morph target image". See [`MorphTargetImage`](crate::morph::MorphTargetImage) for info.
    ///
    /// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
    pub fn try_set_morph_targets(
        &mut self,
        morph_targets: Handle<Image>,
    ) -> Result<(), MeshAccessError> {
        self.morph_targets.replace(Some(morph_targets))?;
        Ok(())
    }

    /// Retrieve the morph targets for this mesh, or None if there are no morph targets.
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_morph_targets`]
    pub fn morph_targets(&self) -> Option<&Handle<Image>> {
        self.morph_targets
            .as_ref_option()
            .expect(MESH_EXTRACTED_ERROR)
    }

    /// Retrieve the morph targets for this mesh, or None if there are no morph targets.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`or
    /// if the morph targets do not exist.
    pub fn try_morph_targets(&self) -> Result<&Handle<Image>, MeshAccessError> {
        self.morph_targets.as_ref()
    }

    /// Consumes the mesh and returns a mesh with the given [morph targets].
    ///
    /// This requires a "morph target image". See [`MorphTargetImage`](crate::morph::MorphTargetImage) for info.
    ///
    /// (Alternatively, you can use [`Mesh::set_morph_targets`] to mutate an existing mesh in-place)
    ///
    /// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_with_morph_targets`]
    #[must_use]
    pub fn with_morph_targets(mut self, morph_targets: Handle<Image>) -> Self {
        self.set_morph_targets(morph_targets);
        self
    }

    /// Consumes the mesh and returns a mesh with the given [morph targets].
    ///
    /// This requires a "morph target image". See [`MorphTargetImage`](crate::morph::MorphTargetImage) for info.
    ///
    /// (Alternatively, you can use [`Mesh::set_morph_targets`] to mutate an existing mesh in-place)
    ///
    /// [morph targets]: https://en.wikipedia.org/wiki/Morph_target_animation
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    pub fn try_with_morph_targets(
        mut self,
        morph_targets: Handle<Image>,
    ) -> Result<Self, MeshAccessError> {
        self.try_set_morph_targets(morph_targets)?;
        Ok(self)
    }

    /// Sets the names of each morph target. This should correspond to the order of the morph targets in `set_morph_targets`.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_set_morph_target_names`]
    pub fn set_morph_target_names(&mut self, names: Vec<String>) {
        self.try_set_morph_target_names(names)
            .expect(MESH_EXTRACTED_ERROR);
    }

    /// Sets the names of each morph target. This should correspond to the order of the morph targets in `set_morph_targets`.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    pub fn try_set_morph_target_names(
        &mut self,
        names: Vec<String>,
    ) -> Result<(), MeshAccessError> {
        self.morph_target_names.replace(Some(names))?;
        Ok(())
    }

    /// Consumes the mesh and returns a mesh with morph target names.
    /// Names should correspond to the order of the morph targets in `set_morph_targets`.
    ///
    /// (Alternatively, you can use [`Mesh::set_morph_target_names`] to mutate an existing mesh in-place)
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_set_morph_target_names`]
    #[must_use]
    pub fn with_morph_target_names(self, names: Vec<String>) -> Self {
        self.try_with_morph_target_names(names)
            .expect(MESH_EXTRACTED_ERROR)
    }

    /// Consumes the mesh and returns a mesh with morph target names.
    /// Names should correspond to the order of the morph targets in `set_morph_targets`.
    ///
    /// (Alternatively, you can use [`Mesh::set_morph_target_names`] to mutate an existing mesh in-place)
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`.
    pub fn try_with_morph_target_names(
        mut self,
        names: Vec<String>,
    ) -> Result<Self, MeshAccessError> {
        self.try_set_morph_target_names(names)?;
        Ok(self)
    }

    /// Gets a list of all morph target names, if they exist.
    ///
    /// # Panics
    /// Panics when the mesh data has already been extracted to `RenderWorld`. To handle
    /// this as an error use [`Mesh::try_morph_target_names`]
    pub fn morph_target_names(&self) -> Option<&[String]> {
        self.try_morph_target_names().expect(MESH_EXTRACTED_ERROR)
    }

    /// Gets a list of all morph target names, if they exist.
    ///
    /// Returns an error if the mesh data has been extracted to `RenderWorld`or
    /// if the morph targets do not exist.
    pub fn try_morph_target_names(&self) -> Result<Option<&[String]>, MeshAccessError> {
        Ok(self
            .morph_target_names
            .as_ref_option()?
            .map(core::ops::Deref::deref))
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

impl core::ops::Mul<Mesh> for Transform {
    type Output = Mesh;

    fn mul(self, rhs: Mesh) -> Self::Output {
        rhs.transformed_by(self)
    }
}

/// A version of [`Mesh`] suitable for serializing for short-term transfer.
///
/// [`Mesh`] does not implement [`Serialize`] / [`Deserialize`] because it is made with the renderer in mind.
/// It is not a general-purpose mesh implementation, and its internals are subject to frequent change.
/// As such, storing a [`Mesh`] on disk is highly discouraged.
///
/// But there are still some valid use cases for serializing a [`Mesh`], namely transferring meshes between processes.
/// To support this, you can create a [`SerializedMesh`] from a [`Mesh`] with [`SerializedMesh::from_mesh`],
/// and then deserialize it with [`SerializedMesh::deserialize`]. The caveats are:
/// - The mesh representation is not valid across different versions of Bevy.
/// - This conversion is lossy. Only the following information is preserved:
///   - Primitive topology
///   - Vertex attributes
///   - Indices
/// - Custom attributes that were not specified with [`MeshDeserializer::add_custom_vertex_attribute`] will be ignored while deserializing.
#[cfg(feature = "serialize")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedMesh {
    primitive_topology: PrimitiveTopology,
    attributes: Vec<(MeshVertexAttributeId, SerializedMeshAttributeData)>,
    indices: Option<Indices>,
}

#[cfg(feature = "serialize")]
impl SerializedMesh {
    /// Create a [`SerializedMesh`] from a [`Mesh`]. See the documentation for [`SerializedMesh`] for caveats.
    pub fn from_mesh(mut mesh: Mesh) -> Self {
        Self {
            primitive_topology: mesh.primitive_topology,
            attributes: mesh
                .attributes
                .replace(None)
                .expect(MESH_EXTRACTED_ERROR)
                .unwrap()
                .into_iter()
                .map(|(id, data)| {
                    (
                        id,
                        SerializedMeshAttributeData::from_mesh_attribute_data(data),
                    )
                })
                .collect(),
            indices: mesh.indices.replace(None).expect(MESH_EXTRACTED_ERROR),
        }
    }

    /// Create a [`Mesh`] from a [`SerializedMesh`]. See the documentation for [`SerializedMesh`] for caveats.
    ///
    /// Use [`MeshDeserializer`] if you need to pass extra options to the deserialization process, such as specifying custom vertex attributes.
    pub fn into_mesh(self) -> Mesh {
        MeshDeserializer::default().deserialize(self)
    }
}

/// Use to specify extra options when deserializing a [`SerializedMesh`] into a [`Mesh`].
#[cfg(feature = "serialize")]
pub struct MeshDeserializer {
    custom_vertex_attributes: HashMap<Box<str>, MeshVertexAttribute>,
}

#[cfg(feature = "serialize")]
impl Default for MeshDeserializer {
    fn default() -> Self {
        // Written like this so that the compiler can validate that we use all the built-in attributes.
        // If you just added a new attribute and got a compile error, please add it to this list :)
        const BUILTINS: [MeshVertexAttribute; Mesh::FIRST_AVAILABLE_CUSTOM_ATTRIBUTE as usize] = [
            Mesh::ATTRIBUTE_POSITION,
            Mesh::ATTRIBUTE_NORMAL,
            Mesh::ATTRIBUTE_UV_0,
            Mesh::ATTRIBUTE_UV_1,
            Mesh::ATTRIBUTE_TANGENT,
            Mesh::ATTRIBUTE_COLOR,
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            Mesh::ATTRIBUTE_JOINT_INDEX,
        ];
        Self {
            custom_vertex_attributes: BUILTINS
                .into_iter()
                .map(|attribute| (attribute.name.into(), attribute))
                .collect(),
        }
    }
}

#[cfg(feature = "serialize")]
impl MeshDeserializer {
    /// Create a new [`MeshDeserializer`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a custom vertex attribute to the deserializer. Custom vertex attributes that were not added with this method will be ignored while deserializing.
    pub fn add_custom_vertex_attribute(
        &mut self,
        name: &str,
        attribute: MeshVertexAttribute,
    ) -> &mut Self {
        self.custom_vertex_attributes.insert(name.into(), attribute);
        self
    }

    /// Deserialize a [`SerializedMesh`] into a [`Mesh`].
    ///
    /// See the documentation for [`SerializedMesh`] for caveats.
    pub fn deserialize(&self, serialized_mesh: SerializedMesh) -> Mesh {
        Mesh {
            attributes: MeshExtractableData::Data(
                serialized_mesh
                .attributes
                .into_iter()
                .filter_map(|(id, data)| {
                    let attribute = data.attribute.clone();
                    let Some(data) =
                        data.try_into_mesh_attribute_data(&self.custom_vertex_attributes)
                    else {
                        warn!(
                            "Deserialized mesh contains custom vertex attribute {attribute:?} that \
                            was not specified with `MeshDeserializer::add_custom_vertex_attribute`. Ignoring."
                        );
                        return None;
                    };
                    Some((id, data))
                })
                .collect()),
            indices: serialized_mesh.indices.into(),
            ..Mesh::new(serialized_mesh.primitive_topology, RenderAssetUsages::default())
        }
    }
}

/// Error that can occur when calling [`Mesh::merge`].
#[derive(Error, Debug, Clone)]
pub enum MeshMergeError {
    #[error("Incompatible vertex attribute types: {} and {}", self_attribute.name, other_attribute.map(|a| a.name).unwrap_or("None"))]
    IncompatibleVertexAttributes {
        self_attribute: MeshVertexAttribute,
        other_attribute: Option<MeshVertexAttribute>,
    },
    #[error(
        "Incompatible primitive topologies: {:?} and {:?}",
        self_primitive_topology,
        other_primitive_topology
    )]
    IncompatiblePrimitiveTopology {
        self_primitive_topology: PrimitiveTopology,
        other_primitive_topology: PrimitiveTopology,
    },
    #[error("Mesh access error: {0}")]
    MeshAccessError(#[from] MeshAccessError),
}

#[cfg(test)]
mod tests {
    use super::Mesh;
    #[cfg(feature = "serialize")]
    use super::SerializedMesh;
    use crate::mesh::{Indices, MeshWindingInvertError, VertexAttributeValues};
    use crate::PrimitiveTopology;
    use bevy_asset::RenderAssetUsages;
    use bevy_math::bounding::Aabb3d;
    use bevy_math::primitives::Triangle3d;
    use bevy_math::Vec3;
    use bevy_transform::components::Transform;

    #[test]
    #[should_panic]
    fn panic_invalid_format() {
        let _mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0, 0.0]]);
    }

    #[test]
    fn transform_mesh() {
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[-1., -1., 2.], [1., -1., 2.], [0., 1., 2.]],
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![
                Vec3::new(-1., -1., 1.).normalize().to_array(),
                Vec3::new(1., -1., 1.).normalize().to_array(),
                [0., 0., 1.],
            ],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.], [1., 0.], [0.5, 1.]]);

        let mesh = mesh.transformed_by(
            Transform::from_translation(Vec3::splat(-2.)).with_scale(Vec3::new(2., 0., -1.)),
        );

        if let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            // All positions are first scaled resulting in `vec![[-2, 0., -2.], [2., 0., -2.], [0., 0., -2.]]`
            // and then shifted by `-2.` along each axis
            assert_eq!(
                positions,
                &vec![[-4.0, -2.0, -4.0], [0.0, -2.0, -4.0], [-2.0, -2.0, -4.0]]
            );
        } else {
            panic!("Mesh does not have a position attribute");
        }

        if let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        {
            assert_eq!(normals, &vec![[0., -1., 0.], [0., -1., 0.], [0., 0., -1.]]);
        } else {
            panic!("Mesh does not have a normal attribute");
        }

        if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
            assert_eq!(uvs, &vec![[0., 0.], [1., 0.], [0.5, 1.]]);
        } else {
            panic!("Mesh does not have a uv attribute");
        }
    }

    #[test]
    fn point_list_mesh_invert_winding() {
        let mesh = Mesh::new(PrimitiveTopology::PointList, RenderAssetUsages::default())
            .with_inserted_indices(Indices::U32(vec![]));
        assert!(matches!(
            mesh.with_inverted_winding(),
            Err(MeshWindingInvertError::WrongTopology)
        ));
    }

    #[test]
    fn line_list_mesh_invert_winding() {
        let mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
            .with_inserted_indices(Indices::U32(vec![0, 1, 1, 2, 2, 3]));
        let mesh = mesh.with_inverted_winding().unwrap();
        assert_eq!(
            mesh.indices().unwrap().iter().collect::<Vec<usize>>(),
            vec![3, 2, 2, 1, 1, 0]
        );
    }

    #[test]
    fn line_list_mesh_invert_winding_fail() {
        let mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
            .with_inserted_indices(Indices::U32(vec![0, 1, 1]));
        assert!(matches!(
            mesh.with_inverted_winding(),
            Err(MeshWindingInvertError::AbruptIndicesEnd)
        ));
    }

    #[test]
    fn line_strip_mesh_invert_winding() {
        let mesh = Mesh::new(PrimitiveTopology::LineStrip, RenderAssetUsages::default())
            .with_inserted_indices(Indices::U32(vec![0, 1, 2, 3]));
        let mesh = mesh.with_inverted_winding().unwrap();
        assert_eq!(
            mesh.indices().unwrap().iter().collect::<Vec<usize>>(),
            vec![3, 2, 1, 0]
        );
    }

    #[test]
    fn triangle_list_mesh_invert_winding() {
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(vec![
            0, 3, 1, // First triangle
            1, 3, 2, // Second triangle
        ]));
        let mesh = mesh.with_inverted_winding().unwrap();
        assert_eq!(
            mesh.indices().unwrap().iter().collect::<Vec<usize>>(),
            vec![
                0, 1, 3, // First triangle
                1, 2, 3, // Second triangle
            ]
        );
    }

    #[test]
    fn triangle_list_mesh_invert_winding_fail() {
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(vec![0, 3, 1, 2]));
        assert!(matches!(
            mesh.with_inverted_winding(),
            Err(MeshWindingInvertError::AbruptIndicesEnd)
        ));
    }

    #[test]
    fn triangle_strip_mesh_invert_winding() {
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleStrip,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(vec![0, 1, 2, 3]));
        let mesh = mesh.with_inverted_winding().unwrap();
        assert_eq!(
            mesh.indices().unwrap().iter().collect::<Vec<usize>>(),
            vec![3, 2, 1, 0]
        );
    }

    #[test]
    fn compute_area_weighted_normals() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );

        //  z      y
        //  |    /
        //  3---2
        //  | /  \
        //  0-----1--x

        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        );
        mesh.insert_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]));
        mesh.compute_area_weighted_normals();
        let normals = mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .unwrap()
            .as_float3()
            .unwrap();
        assert_eq!(4, normals.len());
        // 0
        assert_eq!(Vec3::new(1., 0., 1.).normalize().to_array(), normals[0]);
        // 1
        assert_eq!([0., 0., 1.], normals[1]);
        // 2
        assert_eq!(Vec3::new(1., 0., 1.).normalize().to_array(), normals[2]);
        // 3
        assert_eq!([1., 0., 0.], normals[3]);
    }

    #[test]
    fn compute_area_weighted_normals_proportionate() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );

        //  z      y
        //  |    /
        //  3---2..
        //  | /    \
        //  0-------1---x

        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [2., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        );
        mesh.insert_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]));
        mesh.compute_area_weighted_normals();
        let normals = mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .unwrap()
            .as_float3()
            .unwrap();
        assert_eq!(4, normals.len());
        // 0
        assert_eq!(Vec3::new(1., 0., 2.).normalize().to_array(), normals[0]);
        // 1
        assert_eq!([0., 0., 1.], normals[1]);
        // 2
        assert_eq!(Vec3::new(1., 0., 2.).normalize().to_array(), normals[2]);
        // 3
        assert_eq!([1., 0., 0.], normals[3]);
    }

    #[test]
    fn compute_angle_weighted_normals() {
        // CuboidMeshBuilder duplicates vertices (even though it is indexed)

        //   5---------4
        //  /|        /|
        // 1-+-------0 |
        // | 6-------|-7
        // |/        |/
        // 2---------3
        let verts = vec![
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
        ];

        let indices = Indices::U16(vec![
            0, 1, 2, 2, 3, 0, // front
            5, 4, 7, 7, 6, 5, // back
            1, 5, 6, 6, 2, 1, // left
            4, 0, 3, 3, 7, 4, // right
            4, 5, 1, 1, 0, 4, // top
            3, 2, 6, 6, 7, 3, // bottom
        ]);
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verts);
        mesh.insert_indices(indices);
        mesh.compute_smooth_normals();

        let normals = mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .unwrap()
            .as_float3()
            .unwrap();

        for new in normals.iter().copied().flatten() {
            // std impl is unstable
            const FRAC_1_SQRT_3: f32 = 0.57735026;
            const MIN: f32 = FRAC_1_SQRT_3 - f32::EPSILON;
            const MAX: f32 = FRAC_1_SQRT_3 + f32::EPSILON;
            assert!(new.abs() >= MIN, "{new} < {MIN}");
            assert!(new.abs() <= MAX, "{new} > {MAX}");
        }
    }

    #[test]
    fn triangles_from_triangle_list() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        );
        mesh.insert_indices(Indices::U32(vec![0, 1, 2, 2, 3, 0]));
        assert_eq!(
            vec![
                Triangle3d {
                    vertices: [
                        Vec3::new(0., 0., 0.),
                        Vec3::new(1., 0., 0.),
                        Vec3::new(1., 1., 0.),
                    ]
                },
                Triangle3d {
                    vertices: [
                        Vec3::new(1., 1., 0.),
                        Vec3::new(0., 1., 0.),
                        Vec3::new(0., 0., 0.),
                    ]
                }
            ],
            mesh.triangles().unwrap().collect::<Vec<Triangle3d>>()
        );
    }

    #[test]
    fn triangles_from_triangle_strip() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleStrip,
            RenderAssetUsages::default(),
        );
        // Triangles: (0, 1, 2), (2, 1, 3), (2, 3, 4), (4, 3, 5)
        //
        // 4 - 5
        // | \ |
        // 2 - 3
        // | \ |
        // 0 - 1
        let positions: Vec<Vec3> = [
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [1., 1., 0.],
            [0., 2., 0.],
            [1., 2., 0.],
        ]
        .into_iter()
        .map(Vec3::from_array)
        .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
        mesh.insert_indices(Indices::U32(vec![0, 1, 2, 3, 4, 5]));
        assert_eq!(
            vec![
                Triangle3d {
                    vertices: [positions[0], positions[1], positions[2]]
                },
                Triangle3d {
                    vertices: [positions[2], positions[1], positions[3]]
                },
                Triangle3d {
                    vertices: [positions[2], positions[3], positions[4]]
                },
                Triangle3d {
                    vertices: [positions[4], positions[3], positions[5]]
                },
            ],
            mesh.triangles().unwrap().collect::<Vec<Triangle3d>>()
        );
    }

    #[test]
    fn take_gpu_data_calculates_aabb() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [-0.5, 0., 0.],
                [-1., 0., 0.],
                [-1., -1., 0.],
                [-0.5, -1., 0.],
            ],
        );
        mesh.insert_indices(Indices::U32(vec![0, 1, 2, 2, 3, 0]));
        mesh = mesh.take_gpu_data().unwrap();
        assert_eq!(
            mesh.final_aabb,
            Some(Aabb3d::from_min_max([-1., -1., 0.], [-0.5, 0., 0.]))
        );
    }

    #[cfg(feature = "serialize")]
    #[test]
    fn serialize_deserialize_mesh() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );

        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [2., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        );
        mesh.insert_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]));

        let serialized_mesh = SerializedMesh::from_mesh(mesh.clone());
        let serialized_string = serde_json::to_string(&serialized_mesh).unwrap();
        let serialized_mesh_from_string: SerializedMesh =
            serde_json::from_str(&serialized_string).unwrap();
        let deserialized_mesh = serialized_mesh_from_string.into_mesh();
        assert_eq!(mesh, deserialized_mesh);
    }

    #[test]
    fn deduplicate_vertices() {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        // Quad made of two triangles.
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            // This will be deduplicated.
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            // Position is equal to the first one but UV is different so it won't be deduplicated.
            [0.0, 0.0, 0.0],
        ];
        let uvs = vec![
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            // This will be deduplicated.
            [1.0, 1.0],
            [0.0, 1.0],
            // Use different UV here so it won't be deduplicated.
            [0.0, 0.5],
        ];
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(positions.clone()),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            VertexAttributeValues::Float32x2(uvs.clone()),
        );

        mesh.deduplicate_vertices();
        assert_eq!(6, mesh.indices().unwrap().len());
        // Note we have 5 unique vertices, not 6.
        assert_eq!(5, mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().len());
        assert_eq!(5, mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap().len());

        // Duplicate back.
        mesh.duplicate_vertices();
        assert!(mesh.indices().is_none());
        let VertexAttributeValues::Float32x3(new_positions) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!("Unexpected attribute type")
        };
        let VertexAttributeValues::Float32x2(new_uvs) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap()
        else {
            panic!("Unexpected attribute type")
        };
        assert_eq!(&positions, new_positions);
        assert_eq!(&uvs, new_uvs);
    }
}
