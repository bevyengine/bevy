use crate::render_resource::{
    BufferBindingType, SamplerBindingType, TextureSampleType, TextureViewDimension,
};
use bevy_utils::all_tuples_with_size;
use std::num::{NonZeroU32, NonZeroU64};
use wgpu::{BindingType, ShaderStages, StorageTextureAccess, TextureFormat};

pub struct BindGroupLayoutEntryBuilder {
    pub ty: BindingType,
    pub visibility: Option<ShaderStages>,
    pub count: Option<NonZeroU32>,
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
}

pub struct BindGroupLayoutEntries<const N: usize> {
    entries: [wgpu::BindGroupLayoutEntry; N],
}

impl<const N: usize> BindGroupLayoutEntries<N> {
    #[inline]
    #[allow(unused)]
    pub fn sequential(
        default_visibility: ShaderStages,
        entries_ext: impl IntoBindGroupLayoutEntryBuilderArray<N>,
    ) -> Self {
        let mut i = 0;
        Self {
            entries: entries_ext.into_array().map(|entry| {
                let binding = i;
                i += 1;
                wgpu::BindGroupLayoutEntry {
                    binding,
                    ty: entry.ty,
                    visibility: entry.visibility.unwrap_or(default_visibility),
                    count: entry.count,
                }
            }),
        }
    }

    #[inline]
    #[allow(unused)]
    pub fn with_indices(
        default_visibility: ShaderStages,
        indexed_entries: impl IntoIndexedBindGroupLayoutEntryBuilderArray<N>,
    ) -> Self {
        Self {
            entries: indexed_entries.into_array().map(|(binding, entry)| {
                wgpu::BindGroupLayoutEntry {
                    binding,
                    ty: entry.ty,
                    visibility: entry.visibility.unwrap_or(default_visibility),
                    count: entry.count,
                }
            }),
        }
    }
}

impl<const N: usize> std::ops::Deref for BindGroupLayoutEntries<N> {
    type Target = [wgpu::BindGroupLayoutEntry];
    fn deref(&self) -> &[wgpu::BindGroupLayoutEntry] {
        &self.entries
    }
}

pub trait IntoBindGroupLayoutEntryBuilder {
    fn into_bind_group_layout_entry(self) -> BindGroupLayoutEntryBuilder;
}

impl IntoBindGroupLayoutEntryBuilder for BindingType {
    fn into_bind_group_layout_entry(self) -> BindGroupLayoutEntryBuilder {
        BindGroupLayoutEntryBuilder {
            ty: self,
            visibility: None,
            count: None,
        }
    }
}

impl IntoBindGroupLayoutEntryBuilder for wgpu::BindGroupLayoutEntry {
    fn into_bind_group_layout_entry(self) -> BindGroupLayoutEntryBuilder {
        if self.binding != u32::MAX {
            bevy_log::warn!("The BindGroupLayoutEntries api ignores the binding index when converting a raw wgpu::BindGroupLayoutEntry. You can ignore this warning by setting it to u32::MAX.");
        }
        BindGroupLayoutEntryBuilder {
            ty: self.ty,
            visibility: Some(self.visibility),
            count: self.count,
        }
    }
}

impl IntoBindGroupLayoutEntryBuilder for BindGroupLayoutEntryBuilder {
    fn into_bind_group_layout_entry(self) -> BindGroupLayoutEntryBuilder {
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
                [$($I.into_bind_group_layout_entry(), )*]
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
                [$(($S, $I.into_bind_group_layout_entry())), *]
            }
        }
    }
}
all_tuples_with_size!(impl_to_indexed_binding_type_slice, 1, 32, T, n, s);

#[allow(unused)]
pub fn storage_buffer(
    has_dynamic_offset: bool,
    min_binding_size: Option<NonZeroU64>,
) -> BindGroupLayoutEntryBuilder {
    BindingType::Buffer {
        ty: BufferBindingType::Storage { read_only: false },
        has_dynamic_offset,
        min_binding_size,
    }
    .into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn storage_buffer_read_only(
    has_dynamic_offset: bool,
    min_binding_size: Option<NonZeroU64>,
) -> BindGroupLayoutEntryBuilder {
    BindingType::Buffer {
        ty: BufferBindingType::Storage { read_only: true },
        has_dynamic_offset,
        min_binding_size,
    }
    .into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn uniform_buffer(
    has_dynamic_offset: bool,
    min_binding_size: Option<NonZeroU64>,
) -> BindGroupLayoutEntryBuilder {
    BindingType::Buffer {
        ty: BufferBindingType::Uniform,
        has_dynamic_offset,
        min_binding_size,
    }
    .into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_2d(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
    BindingType::Texture {
        sample_type,
        view_dimension: TextureViewDimension::D2,
        multisampled: false,
    }
    .into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_2d_multisampled(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
    BindingType::Texture {
        sample_type,
        view_dimension: TextureViewDimension::D2,
        multisampled: true,
    }
    .into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_2d_array(sample_type: TextureSampleType) -> BindGroupLayoutEntryBuilder {
    BindingType::Texture {
        sample_type,
        view_dimension: TextureViewDimension::D2Array,
        multisampled: false,
    }
    .into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_2d_array_multisampled(
    sample_type: TextureSampleType,
) -> BindGroupLayoutEntryBuilder {
    BindingType::Texture {
        sample_type,
        view_dimension: TextureViewDimension::D2Array,
        multisampled: true,
    }
    .into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_2d_f32(filterable: bool) -> BindGroupLayoutEntryBuilder {
    texture_2d(TextureSampleType::Float { filterable }).into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_2d_multisampled_f32(filterable: bool) -> BindGroupLayoutEntryBuilder {
    texture_2d_multisampled(TextureSampleType::Float { filterable }).into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_2d_i32() -> BindGroupLayoutEntryBuilder {
    texture_2d(TextureSampleType::Sint).into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_2d_multisampled_i32() -> BindGroupLayoutEntryBuilder {
    texture_2d_multisampled(TextureSampleType::Sint).into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_2d_u32() -> BindGroupLayoutEntryBuilder {
    texture_2d(TextureSampleType::Uint).into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_2d_multisampled_u32() -> BindGroupLayoutEntryBuilder {
    texture_2d_multisampled(TextureSampleType::Uint).into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_depth_2d() -> BindGroupLayoutEntryBuilder {
    texture_2d(TextureSampleType::Depth).into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_depth_2d_multisampled() -> BindGroupLayoutEntryBuilder {
    texture_2d_multisampled(TextureSampleType::Depth).into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn sampler(sampler_binding_type: SamplerBindingType) -> BindGroupLayoutEntryBuilder {
    BindingType::Sampler(sampler_binding_type).into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_storage_2d(
    format: TextureFormat,
    access: StorageTextureAccess,
) -> BindGroupLayoutEntryBuilder {
    BindingType::StorageTexture {
        access,
        format,
        view_dimension: TextureViewDimension::D2,
    }
    .into_bind_group_layout_entry()
}

#[allow(unused)]
pub fn texture_storage_2d_array(
    format: TextureFormat,
    access: StorageTextureAccess,
) -> BindGroupLayoutEntryBuilder {
    BindingType::StorageTexture {
        access,
        format,
        view_dimension: TextureViewDimension::D2Array,
    }
    .into_bind_group_layout_entry()
}
