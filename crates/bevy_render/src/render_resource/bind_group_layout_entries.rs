use bevy_utils::all_tuples_with_size;
use std::num::NonZeroU32;
use wgpu::{BindGroupLayoutEntry, BindingType, ShaderStages};

/// Helper for constructing bind group layouts.
///
/// Allows constructing the layout's entries as:
/// ```ignore (render_device cannot be easily accessed)
/// let layout = render_device.create_bind_group_layout(
///     "my_bind_group_layout",
///     &BindGroupLayoutEntries::with_indices(
///         // The layout entries will only be visible in the fragment stage
///         ShaderStages::FRAGMENT,
///         (
///             // Screen texture
///             (2, texture_2d(TextureSampleType::Float { filterable: true })),
///             // Sampler
///             (3, sampler(SamplerBindingType::Filtering)),
///         ),
///     ),
/// );
/// ```
///
/// instead of
///
/// ```ignore (render_device cannot be easily accessed)
/// let layout = render_device.create_bind_group_layout(
///     "my_bind_group_layout",
///     &[
///         // Screen texture
///         BindGroupLayoutEntry {
///             binding: 2,
///             visibility: ShaderStages::FRAGMENT,
///             ty: BindingType::Texture {
///                 sample_type: TextureSampleType::Float { filterable: true },
///                 view_dimension: TextureViewDimension::D2,
///                 multisampled: false,
///             },
///             count: None,
///         },
///         // Sampler
///         BindGroupLayoutEntry {
///             binding: 3,
///             visibility: ShaderStages::FRAGMENT,
///             ty: BindingType::Sampler(SamplerBindingType::Filtering),
///             count: None,
///         },
///     ],
/// );
/// ```
///
/// or
///
/// ```ignore (render_device cannot be easily accessed)
/// render_device.create_bind_group_layout(
///     "my_bind_group_layout",
///     &BindGroupLayoutEntries::sequential(
///         ShaderStages::FRAGMENT,
///         (
///             // Screen texture
///             texture_2d(TextureSampleType::Float { filterable: true }),
///             // Sampler
///             sampler(SamplerBindingType::Filtering),
///         ),
///     ),
/// );
/// ```
///
/// instead of
///
/// ```ignore (render_device cannot be easily accessed)
/// let layout = render_device.create_bind_group_layout(
///     "my_bind_group_layout",
///     &[
///         // Screen texture
///         BindGroupLayoutEntry {
///             binding: 0,
///             visibility: ShaderStages::FRAGMENT,
///             ty: BindingType::Texture {
///                 sample_type: TextureSampleType::Float { filterable: true },
///                 view_dimension: TextureViewDimension::D2,
///                 multisampled: false,
///             },
///             count: None,
///         },
///         // Sampler
///         BindGroupLayoutEntry {
///             binding: 1,
///             visibility: ShaderStages::FRAGMENT,
///             ty: BindingType::Sampler(SamplerBindingType::Filtering),
///             count: None,
///         },
///     ],
/// );
/// ```
///
/// or
///
/// ```ignore (render_device cannot be easily accessed)
/// render_device.create_bind_group_layout(
///     "my_bind_group_layout",
///     &BindGroupLayoutEntries::single(
///         ShaderStages::FRAGMENT,
///         texture_2d(TextureSampleType::Float { filterable: true }),
///     ),
/// );
/// ```
///
/// instead of
///
/// ```ignore (render_device cannot be easily accessed)
/// let layout = render_device.create_bind_group_layout(
///     "my_bind_group_layout",
///     &[
///         BindGroupLayoutEntry {
///             binding: 0,
///             visibility: ShaderStages::FRAGMENT,
///             ty: BindingType::Texture {
///                 sample_type: TextureSampleType::Float { filterable: true },
///                 view_dimension: TextureViewDimension::D2,
///                 multisampled: false,
///             },
///             count: None,
///         },
///     ],
/// );
/// ```

#[derive(Clone, Copy)]
pub struct BindGroupLayoutEntryBuilder {
    ty: BindingType,
    visibility: Option<ShaderStages>,
    count: Option<NonZeroU32>,
}

impl BindGroupLayoutEntryBuilder {
    pub fn visibility(mut self, visibility: ShaderStages) -> Self {
        self.visibility = Some(visibility);
        self
    }

    pub fn count(mut self, count: NonZeroU32) -> Self {
        self.count = Some(count);
        self
    }

    pub fn build(&self, binding: u32, default_visibility: ShaderStages) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            ty: self.ty,
            visibility: self.visibility.unwrap_or(default_visibility),
            count: self.count,
        }
    }
}

pub struct BindGroupLayoutEntries<const N: usize> {
    entries: [BindGroupLayoutEntry; N],
}

impl<const N: usize> BindGroupLayoutEntries<N> {
    #[inline]
    pub fn sequential(
        default_visibility: ShaderStages,
        entries_ext: impl IntoBindGroupLayoutEntryBuilderArray<N>,
    ) -> Self {
        let mut i = 0;
        Self {
            entries: entries_ext.into_array().map(|entry| {
                let binding = i;
                i += 1;
                entry.build(binding, default_visibility)
            }),
        }
    }

    #[inline]
    pub fn with_indices(
        default_visibility: ShaderStages,
        indexed_entries: impl IntoIndexedBindGroupLayoutEntryBuilderArray<N>,
    ) -> Self {
        Self {
            entries: indexed_entries
                .into_array()
                .map(|(binding, entry)| entry.build(binding, default_visibility)),
        }
    }
}

impl BindGroupLayoutEntries<1> {
    pub fn single(
        visibility: ShaderStages,
        resource: impl IntoBindGroupLayoutEntryBuilder,
    ) -> [BindGroupLayoutEntry; 1] {
        [resource
            .into_bind_group_layout_entry_builder()
            .build(0, visibility)]
    }
}

impl<const N: usize> std::ops::Deref for BindGroupLayoutEntries<N> {
    type Target = [BindGroupLayoutEntry];
    fn deref(&self) -> &[BindGroupLayoutEntry] {
        &self.entries
    }
}

pub trait IntoBindGroupLayoutEntryBuilder {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder;
}

impl IntoBindGroupLayoutEntryBuilder for BindingType {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindGroupLayoutEntryBuilder {
            ty: self,
            visibility: None,
            count: None,
        }
    }
}

impl IntoBindGroupLayoutEntryBuilder for BindGroupLayoutEntry {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        if self.binding != u32::MAX {
            bevy_utils::tracing::warn!("The BindGroupLayoutEntries api ignores the binding index when converting a raw wgpu::BindGroupLayoutEntry. You can ignore this warning by setting it to u32::MAX.");
        }
        BindGroupLayoutEntryBuilder {
            ty: self.ty,
            visibility: Some(self.visibility),
            count: self.count,
        }
    }
}

impl IntoBindGroupLayoutEntryBuilder for BindGroupLayoutEntryBuilder {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        self
    }
}

pub trait IntoBindGroupLayoutEntryBuilderArray<const N: usize> {
    fn into_array(self) -> [BindGroupLayoutEntryBuilder; N];
}
macro_rules! impl_to_binding_type_slice {
    ($N: expr, $(($T: ident, $I: ident)),*) => {
        impl<$($T: IntoBindGroupLayoutEntryBuilder),*> IntoBindGroupLayoutEntryBuilderArray<$N> for ($($T,)*) {
            #[inline]
            fn into_array(self) -> [BindGroupLayoutEntryBuilder; $N] {
                let ($($I,)*) = self;
                [$($I.into_bind_group_layout_entry_builder(), )*]
            }
        }
    }
}
all_tuples_with_size!(impl_to_binding_type_slice, 1, 32, T, s);

pub trait IntoIndexedBindGroupLayoutEntryBuilderArray<const N: usize> {
    fn into_array(self) -> [(u32, BindGroupLayoutEntryBuilder); N];
}
macro_rules! impl_to_indexed_binding_type_slice {
    ($N: expr, $(($T: ident, $S: ident, $I: ident)),*) => {
        impl<$($T: IntoBindGroupLayoutEntryBuilder),*> IntoIndexedBindGroupLayoutEntryBuilderArray<$N> for ($((u32, $T),)*) {
            #[inline]
            fn into_array(self) -> [(u32, BindGroupLayoutEntryBuilder); $N] {
                let ($(($S, $I),)*) = self;
                [$(($S, $I.into_bind_group_layout_entry_builder())), *]
            }
        }
    }
}
all_tuples_with_size!(impl_to_indexed_binding_type_slice, 1, 32, T, n, s);

impl<const N: usize> IntoBindGroupLayoutEntryBuilderArray<N> for [BindGroupLayoutEntry; N] {
    fn into_array(self) -> [BindGroupLayoutEntryBuilder; N] {
        self.map(|x| x.into_bind_group_layout_entry_builder())
    }
}

pub struct DynamicBindGroupLayoutEntries {
    default_visibility: ShaderStages,
    entries: Vec<BindGroupLayoutEntry>,
}

impl DynamicBindGroupLayoutEntries {
    pub fn sequential<const N: usize>(
        default_visibility: ShaderStages,
        entries: impl IntoBindGroupLayoutEntryBuilderArray<N>,
    ) -> Self {
        Self {
            default_visibility,
            entries: entries
                .into_array()
                .into_iter()
                .enumerate()
                .map(|(ix, resource)| resource.build(ix as u32, default_visibility))
                .collect(),
        }
    }

    pub fn extend_sequential<const N: usize>(
        mut self,
        entries: impl IntoBindGroupLayoutEntryBuilderArray<N>,
    ) -> Self {
        let start = self.entries.last().unwrap().binding + 1;
        self.entries.extend(
            entries
                .into_array()
                .into_iter()
                .enumerate()
                .map(|(ix, resource)| resource.build(start + ix as u32, self.default_visibility)),
        );
        self
    }

    pub fn new_with_indices<const N: usize>(
        default_visibility: ShaderStages,
        entries: impl IntoIndexedBindGroupLayoutEntryBuilderArray<N>,
    ) -> Self {
        Self {
            default_visibility,
            entries: entries
                .into_array()
                .into_iter()
                .map(|(binding, resource)| resource.build(binding, default_visibility))
                .collect(),
        }
    }

    pub fn extend_with_indices<const N: usize>(
        mut self,
        entries: impl IntoIndexedBindGroupLayoutEntryBuilderArray<N>,
    ) -> Self {
        self.entries.extend(
            entries
                .into_array()
                .into_iter()
                .map(|(binding, resource)| resource.build(binding, self.default_visibility)),
        );
        self
    }
}

impl std::ops::Deref for DynamicBindGroupLayoutEntries {
    type Target = [BindGroupLayoutEntry];

    fn deref(&self) -> &[BindGroupLayoutEntry] {
        &self.entries
    }
}

pub mod binding_types {
    use crate::render_resource::{
        BufferBindingType, SamplerBindingType, TextureSampleType, TextureViewDimension,
    };
    use encase::ShaderType;
    use std::num::NonZeroU64;
    use wgpu::{StorageTextureAccess, TextureFormat};

    use super::*;

    pub fn storage_buffer<T: ShaderType>(has_dynamic_offset: bool) -> BindGroupLayoutEntryBuilder {
        storage_buffer_sized(has_dynamic_offset, Some(T::min_size()))
    }

    pub fn storage_buffer_sized(
        has_dynamic_offset: bool,
        min_binding_size: Option<NonZeroU64>,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset,
            min_binding_size,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn storage_buffer_read_only<T: ShaderType>(
        has_dynamic_offset: bool,
    ) -> BindGroupLayoutEntryBuilder {
        storage_buffer_read_only_sized(has_dynamic_offset, Some(T::min_size()))
    }

    pub fn storage_buffer_read_only_sized(
        has_dynamic_offset: bool,
        min_binding_size: Option<NonZeroU64>,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset,
            min_binding_size,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn uniform_buffer<T: ShaderType>(has_dynamic_offset: bool) -> BindGroupLayoutEntryBuilder {
        uniform_buffer_sized(has_dynamic_offset, Some(T::min_size()))
    }

    pub fn uniform_buffer_sized(
        has_dynamic_offset: bool,
        min_binding_size: Option<NonZeroU64>,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset,
            min_binding_size,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_1d(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D1,
            multisampled: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_2d(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_2d_multisampled(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D2,
            multisampled: true,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_2d_array(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D2Array,
            multisampled: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_2d_array_multisampled(
        sample_type: TextureSampleType,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D2Array,
            multisampled: true,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_depth_2d() -> BindGroupLayoutEntryBuilder {
        texture_2d(TextureSampleType::Depth).into_bind_group_layout_entry_builder()
    }

    pub fn texture_depth_2d_multisampled() -> BindGroupLayoutEntryBuilder {
        texture_2d_multisampled(TextureSampleType::Depth).into_bind_group_layout_entry_builder()
    }

    pub fn texture_cube(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::Cube,
            multisampled: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_cube_multisampled(
        sample_type: TextureSampleType,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::Cube,
            multisampled: true,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_cube_array(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::CubeArray,
            multisampled: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_cube_array_multisampled(
        sample_type: TextureSampleType,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::CubeArray,
            multisampled: true,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_3d(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D3,
            multisampled: false,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_3d_multisampled(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type,
            view_dimension: TextureViewDimension::D3,
            multisampled: true,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn sampler(sampler_binding_type: SamplerBindingType) -> BindGroupLayoutEntryBuilder {
        BindingType::Sampler(sampler_binding_type).into_bind_group_layout_entry_builder()
    }

    pub fn texture_storage_2d(
        format: TextureFormat,
        access: StorageTextureAccess,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::StorageTexture {
            access,
            format,
            view_dimension: TextureViewDimension::D2,
        }
        .into_bind_group_layout_entry_builder()
    }

    pub fn texture_storage_2d_array(
        format: TextureFormat,
        access: StorageTextureAccess,
    ) -> BindGroupLayoutEntryBuilder {
        BindingType::StorageTexture {
            access,
            format,
            view_dimension: TextureViewDimension::D2Array,
        }
        .into_bind_group_layout_entry_builder()
    }
}
