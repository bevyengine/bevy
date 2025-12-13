//! Types and functions relating to bindless resources.

use alloc::borrow::Cow;
use core::{
    num::{NonZeroU32, NonZeroU64},
    ops::Range,
};

use bevy_derive::{Deref, DerefMut};
use wgpu::{
    BindGroupLayoutEntry, SamplerBindingType, ShaderStages, TextureSampleType, TextureViewDimension,
};

use crate::render_resource::binding_types::storage_buffer_read_only_sized;

use super::binding_types::{
    sampler, texture_1d, texture_2d, texture_2d_array, texture_3d, texture_cube, texture_cube_array,
};

/// The default value for the number of resources that can be stored in a slab
/// on this platform.
///
/// See the documentation for [`BindlessSlabResourceLimit`] for more
/// information.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub const AUTO_BINDLESS_SLAB_RESOURCE_LIMIT: u32 = 64;
/// The default value for the number of resources that can be stored in a slab
/// on this platform.
///
/// See the documentation for [`BindlessSlabResourceLimit`] for more
/// information.
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub const AUTO_BINDLESS_SLAB_RESOURCE_LIMIT: u32 = 2048;

/// The binding numbers for the built-in binding arrays of each bindless
/// resource type.
///
/// In the case of materials, the material allocator manages these binding
/// arrays.
///
/// `bindless.wgsl` contains declarations of these arrays for use in your
/// shaders. If you change these, make sure to update that file as well.
pub static BINDING_NUMBERS: [(BindlessResourceType, BindingNumber); 9] = [
    (BindlessResourceType::SamplerFiltering, BindingNumber(1)),
    (BindlessResourceType::SamplerNonFiltering, BindingNumber(2)),
    (BindlessResourceType::SamplerComparison, BindingNumber(3)),
    (BindlessResourceType::Texture1d, BindingNumber(4)),
    (BindlessResourceType::Texture2d, BindingNumber(5)),
    (BindlessResourceType::Texture2dArray, BindingNumber(6)),
    (BindlessResourceType::Texture3d, BindingNumber(7)),
    (BindlessResourceType::TextureCube, BindingNumber(8)),
    (BindlessResourceType::TextureCubeArray, BindingNumber(9)),
];

/// The maximum number of resources that can be stored in a slab.
///
/// This limit primarily exists in order to work around `wgpu` performance
/// problems involving large numbers of bindless resources. Also, some
/// platforms, such as Metal, currently enforce limits on the number of
/// resources in use.
///
/// This corresponds to `LIMIT` in the `#[bindless(LIMIT)]` attribute when
/// deriving [`crate::render_resource::AsBindGroup`].
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum BindlessSlabResourceLimit {
    /// Allows the renderer to choose a reasonable value for the resource limit
    /// based on the platform.
    ///
    /// This value has been tuned, so you should default to this value unless
    /// you have special platform-specific considerations that prevent you from
    /// using it.
    #[default]
    Auto,

    /// A custom value for the resource limit.
    ///
    /// Bevy will allocate no more than this number of resources in a slab,
    /// unless exceeding this value is necessary in order to allocate at all
    /// (i.e. unless the number of bindless resources in your bind group exceeds
    /// this value), in which case Bevy can exceed it.
    Custom(u32),
}

/// Information about the bindless resources in this object.
///
/// The material bind group allocator uses this descriptor in order to create
/// and maintain bind groups. The fields within this bindless descriptor are
/// [`Cow`]s in order to support both the common case in which the fields are
/// simply `static` constants and the more unusual case in which the fields are
/// dynamically generated efficiently. An example of the latter case is
/// `ExtendedMaterial`, which needs to assemble a bindless descriptor from those
/// of the base material and the material extension at runtime.
///
/// This structure will only be present if this object is bindless.
pub struct BindlessDescriptor {
    /// The bindless resource types that this object uses, in order of bindless
    /// index.
    ///
    /// The resource assigned to binding index 0 will be at index 0, the
    /// resource assigned to binding index will be at index 1 in this array, and
    /// so on. Unused binding indices are set to [`BindlessResourceType::None`].
    pub resources: Cow<'static, [BindlessResourceType]>,
    /// The [`BindlessBufferDescriptor`] for each bindless buffer that this
    /// object uses.
    ///
    /// The order of this array is irrelevant.
    pub buffers: Cow<'static, [BindlessBufferDescriptor]>,
    /// The [`BindlessIndexTableDescriptor`]s describing each bindless index
    /// table.
    ///
    /// This list must be sorted by the first bindless index.
    pub index_tables: Cow<'static, [BindlessIndexTableDescriptor]>,
}

/// The type of potentially-bindless resource.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum BindlessResourceType {
    /// No bindless resource.
    ///
    /// This is used as a placeholder to fill holes in the
    /// [`BindlessDescriptor::resources`] list.
    None,
    /// A storage buffer.
    Buffer,
    /// A filtering sampler.
    SamplerFiltering,
    /// A non-filtering sampler (nearest neighbor).
    SamplerNonFiltering,
    /// A comparison sampler (typically used for shadow maps).
    SamplerComparison,
    /// A 1D texture.
    Texture1d,
    /// A 2D texture.
    Texture2d,
    /// A 2D texture array.
    ///
    /// Note that this differs from a binding array. 2D texture arrays must all
    /// have the same size and format.
    Texture2dArray,
    /// A 3D texture.
    Texture3d,
    /// A cubemap texture.
    TextureCube,
    /// A cubemap texture array.
    ///
    /// Note that this differs from a binding array. Cubemap texture arrays must
    /// all have the same size and format.
    TextureCubeArray,
    /// Multiple instances of plain old data concatenated into a single buffer.
    ///
    /// This corresponds to the `#[data]` declaration in
    /// [`crate::render_resource::AsBindGroup`].
    ///
    /// Note that this resource doesn't itself map to a GPU-level binding
    /// resource and instead depends on the `MaterialBindGroupAllocator` to
    /// create a binding resource for it.
    DataBuffer,
}

/// Describes a bindless buffer.
///
/// Unlike samplers and textures, each buffer in a bind group gets its own
/// unique bind group entry. That is, there isn't any `bindless_buffers` binding
/// array to go along with `bindless_textures_2d`,
/// `bindless_samplers_filtering`, etc. Therefore, this descriptor contains two
/// indices: the *binding number* and the *bindless index*. The binding number
/// is the `@binding` number used in the shader, while the bindless index is the
/// index of the buffer in the bindless index table (which is itself
/// conventionally bound to binding number 0).
///
/// When declaring the buffer in a derived implementation
/// [`crate::render_resource::AsBindGroup`] with syntax like
/// `#[uniform(BINDLESS_INDEX, StandardMaterialUniform,
/// bindless(BINDING_NUMBER)]`, the bindless index is `BINDLESS_INDEX`, and the
/// binding number is `BINDING_NUMBER`. Note the order.
#[derive(Clone, Copy, Debug)]
pub struct BindlessBufferDescriptor {
    /// The actual binding number of the buffer.
    ///
    /// This is declared with `@binding` in WGSL. When deriving
    /// [`crate::render_resource::AsBindGroup`], this is the `BINDING_NUMBER` in
    /// `#[uniform(BINDLESS_INDEX, StandardMaterialUniform,
    /// bindless(BINDING_NUMBER)]`.
    pub binding_number: BindingNumber,
    /// The index of the buffer in the bindless index table.
    ///
    /// In the shader, this is the index into the table bound to binding 0. When
    /// deriving [`crate::render_resource::AsBindGroup`], this is the
    /// `BINDLESS_INDEX` in `#[uniform(BINDLESS_INDEX, StandardMaterialUniform,
    /// bindless(BINDING_NUMBER)]`.
    pub bindless_index: BindlessIndex,
    /// The size of the buffer in bytes, if known.
    pub size: Option<usize>,
}

/// Describes the layout of the bindless index table, which maps bindless
/// indices to indices within the binding arrays.
#[derive(Clone)]
pub struct BindlessIndexTableDescriptor {
    /// The range of bindless indices that this descriptor covers.
    pub indices: Range<BindlessIndex>,
    /// The binding at which the index table itself will be bound.
    ///
    /// By default, this is binding 0, but it can be changed with the
    /// `#[bindless(index_table(binding(B)))]` attribute.
    pub binding_number: BindingNumber,
}

/// The index of the actual binding in the bind group.
///
/// This is the value specified in WGSL as `@binding`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Deref, DerefMut)]
pub struct BindingNumber(pub u32);

/// The index in the bindless index table.
///
/// This table is conventionally bound to binding number 0.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Hash, Debug, Deref, DerefMut)]
pub struct BindlessIndex(pub u32);

/// Creates the bind group layout entries common to all shaders that use
/// bindless bind groups.
///
/// `bindless_resource_count` specifies the total number of bindless resources.
/// `bindless_slab_resource_limit` specifies the resolved
/// [`BindlessSlabResourceLimit`] value.
pub fn create_bindless_bind_group_layout_entries(
    bindless_index_table_length: u32,
    bindless_slab_resource_limit: u32,
    bindless_index_table_binding_number: BindingNumber,
) -> Vec<BindGroupLayoutEntry> {
    let bindless_slab_resource_limit =
        NonZeroU32::new(bindless_slab_resource_limit).expect("Bindless slot count must be nonzero");

    // The maximum size of a binding array is the
    // `bindless_slab_resource_limit`, which would occur if all of the bindless
    // resources were of the same type. So we create our binding arrays with
    // that size.

    vec![
        // Start with the bindless index table, bound to binding number 0.
        storage_buffer_read_only_sized(
            false,
            NonZeroU64::new(bindless_index_table_length as u64 * size_of::<u32>() as u64),
        )
        .build(
            *bindless_index_table_binding_number,
            ShaderStages::FRAGMENT | ShaderStages::VERTEX | ShaderStages::COMPUTE,
        ),
        // Continue with the common bindless resource arrays.
        sampler(SamplerBindingType::Filtering)
            .count(bindless_slab_resource_limit)
            .build(
                1,
                ShaderStages::FRAGMENT | ShaderStages::VERTEX | ShaderStages::COMPUTE,
            ),
        sampler(SamplerBindingType::NonFiltering)
            .count(bindless_slab_resource_limit)
            .build(
                2,
                ShaderStages::FRAGMENT | ShaderStages::VERTEX | ShaderStages::COMPUTE,
            ),
        sampler(SamplerBindingType::Comparison)
            .count(bindless_slab_resource_limit)
            .build(
                3,
                ShaderStages::FRAGMENT | ShaderStages::VERTEX | ShaderStages::COMPUTE,
            ),
        texture_1d(TextureSampleType::Float { filterable: true })
            .count(bindless_slab_resource_limit)
            .build(
                4,
                ShaderStages::FRAGMENT | ShaderStages::VERTEX | ShaderStages::COMPUTE,
            ),
        texture_2d(TextureSampleType::Float { filterable: true })
            .count(bindless_slab_resource_limit)
            .build(
                5,
                ShaderStages::FRAGMENT | ShaderStages::VERTEX | ShaderStages::COMPUTE,
            ),
        texture_2d_array(TextureSampleType::Float { filterable: true })
            .count(bindless_slab_resource_limit)
            .build(
                6,
                ShaderStages::FRAGMENT | ShaderStages::VERTEX | ShaderStages::COMPUTE,
            ),
        texture_3d(TextureSampleType::Float { filterable: true })
            .count(bindless_slab_resource_limit)
            .build(
                7,
                ShaderStages::FRAGMENT | ShaderStages::VERTEX | ShaderStages::COMPUTE,
            ),
        texture_cube(TextureSampleType::Float { filterable: true })
            .count(bindless_slab_resource_limit)
            .build(
                8,
                ShaderStages::FRAGMENT | ShaderStages::VERTEX | ShaderStages::COMPUTE,
            ),
        texture_cube_array(TextureSampleType::Float { filterable: true })
            .count(bindless_slab_resource_limit)
            .build(
                9,
                ShaderStages::FRAGMENT | ShaderStages::VERTEX | ShaderStages::COMPUTE,
            ),
    ]
}

impl BindlessSlabResourceLimit {
    /// Determines the actual bindless slab resource limit on this platform.
    pub fn resolve(&self) -> u32 {
        match *self {
            BindlessSlabResourceLimit::Auto => AUTO_BINDLESS_SLAB_RESOURCE_LIMIT,
            BindlessSlabResourceLimit::Custom(limit) => limit,
        }
    }
}

impl BindlessResourceType {
    /// Returns the binding number for the common array of this resource type.
    ///
    /// For example, if you pass `BindlessResourceType::Texture2d`, this will
    /// return 5, in order to match the `@group(2) @binding(5) var
    /// bindless_textures_2d: binding_array<texture_2d<f32>>` declaration in
    /// `bindless.wgsl`.
    ///
    /// Not all resource types have fixed binding numbers. If you call
    /// [`Self::binding_number`] on such a resource type, it returns `None`.
    ///
    /// Note that this returns a static reference to the binding number, not the
    /// binding number itself. This is to conform to an idiosyncratic API in
    /// `wgpu` whereby binding numbers for binding arrays are taken by `&u32`
    /// *reference*, not by `u32` value.
    pub fn binding_number(&self) -> Option<&'static BindingNumber> {
        match BINDING_NUMBERS.binary_search_by_key(self, |(key, _)| *key) {
            Ok(binding_number) => Some(&BINDING_NUMBERS[binding_number].1),
            Err(_) => None,
        }
    }
}

impl From<TextureViewDimension> for BindlessResourceType {
    fn from(texture_view_dimension: TextureViewDimension) -> Self {
        match texture_view_dimension {
            TextureViewDimension::D1 => BindlessResourceType::Texture1d,
            TextureViewDimension::D2 => BindlessResourceType::Texture2d,
            TextureViewDimension::D2Array => BindlessResourceType::Texture2dArray,
            TextureViewDimension::Cube => BindlessResourceType::TextureCube,
            TextureViewDimension::CubeArray => BindlessResourceType::TextureCubeArray,
            TextureViewDimension::D3 => BindlessResourceType::Texture3d,
        }
    }
}

impl From<SamplerBindingType> for BindlessResourceType {
    fn from(sampler_binding_type: SamplerBindingType) -> Self {
        match sampler_binding_type {
            SamplerBindingType::Filtering => BindlessResourceType::SamplerFiltering,
            SamplerBindingType::NonFiltering => BindlessResourceType::SamplerNonFiltering,
            SamplerBindingType::Comparison => BindlessResourceType::SamplerComparison,
        }
    }
}

impl From<u32> for BindlessIndex {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<u32> for BindingNumber {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
