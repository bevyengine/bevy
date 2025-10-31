use crate::render_resource::{
    BindGroupLayoutEntryBuilder, Buffer, IntoBindGroupLayoutEntryBuilder, IntoBinding, TextureView,
};
use bevy_derive::Deref;
use wgpu::{
    BindingResource, BindingType, BufferBindingType, StorageTextureAccess, TextureViewDimension,
};

/// Corresponds to `var<storage, read> my_buffer: T` in a WGSL shader.
#[derive(Clone, Deref)]
pub struct StorageBufferReadOnly<'a>(pub &'a Buffer);

/// Corresponds to `var<storage, read_write> my_buffer: T` in a WGSL shader.
#[derive(Clone, Deref)]
pub struct StorageBufferReadWrite<'a>(pub &'a Buffer);

/// Corresponds to `var my_texture: texture_2d<T>` in a WGSL shader.
#[derive(Clone, Deref)]
pub struct SampledTexture<'a>(pub &'a TextureView);

/// Corresponds to `var my_texture: texture_storage_2d<F, read_write>` in a WGSL shader.
#[derive(Clone, Deref)]
pub struct StorageTextureReadWrite<'a>(pub &'a TextureView);

/// Corresponds to `var my_texture: texture_storage_2d<F, write>` in a WGSL shader.
#[derive(Clone, Deref)]
pub struct StorageTextureWriteOnly<'a>(pub &'a TextureView);

/// Corresponds to `var my_texture: texture_storage_2d<F, read>` in a WGSL shader.
#[derive(Clone, Deref)]
pub struct StorageTextureReadOnly<'a>(pub &'a TextureView);

/// Corresponds to `var my_texture: texture_storage_2d<F, atomic>` in a WGSL shader.
#[derive(Clone, Deref)]
pub struct StorageTextureAtomic<'a>(pub &'a TextureView);

impl<'a> IntoBinding<'a> for StorageBufferReadOnly<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        self.0.as_entire_binding()
    }
}

impl<'a> IntoBindGroupLayoutEntryBuilder for StorageBufferReadOnly<'a> {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
        .into_bind_group_layout_entry_builder()
    }
}

impl<'a> IntoBinding<'a> for StorageBufferReadWrite<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        self.0.as_entire_binding()
    }
}

impl<'a> IntoBindGroupLayoutEntryBuilder for StorageBufferReadWrite<'a> {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
        .into_bind_group_layout_entry_builder()
    }
}

impl<'a> IntoBinding<'a> for SampledTexture<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        BindingResource::TextureView(self.0)
    }
}

impl<'a> IntoBindGroupLayoutEntryBuilder for SampledTexture<'a> {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type: self.texture().format().sample_type(None, None).unwrap(),
            view_dimension: TextureViewDimension::D2,
            multisampled: self.0.texture().sample_count() > 1,
        }
        .into_bind_group_layout_entry_builder()
    }
}

impl<'a> IntoBinding<'a> for StorageTextureReadWrite<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        BindingResource::TextureView(self.0)
    }
}

impl<'a> IntoBindGroupLayoutEntryBuilder for StorageTextureReadWrite<'a> {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindingType::StorageTexture {
            access: StorageTextureAccess::ReadWrite,
            format: self.0.texture().format(),
            view_dimension: TextureViewDimension::D2,
        }
        .into_bind_group_layout_entry_builder()
    }
}

impl<'a> IntoBinding<'a> for StorageTextureWriteOnly<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        BindingResource::TextureView(self.0)
    }
}

impl<'a> IntoBindGroupLayoutEntryBuilder for StorageTextureWriteOnly<'a> {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindingType::StorageTexture {
            access: StorageTextureAccess::WriteOnly,
            format: self.0.texture().format(),
            view_dimension: TextureViewDimension::D2,
        }
        .into_bind_group_layout_entry_builder()
    }
}

impl<'a> IntoBinding<'a> for StorageTextureReadOnly<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        BindingResource::TextureView(self.0)
    }
}

impl<'a> IntoBindGroupLayoutEntryBuilder for StorageTextureReadOnly<'a> {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindingType::StorageTexture {
            access: StorageTextureAccess::ReadOnly,
            format: self.0.texture().format(),
            view_dimension: TextureViewDimension::D2,
        }
        .into_bind_group_layout_entry_builder()
    }
}

impl<'a> IntoBinding<'a> for StorageTextureAtomic<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        BindingResource::TextureView(self.0)
    }
}

impl<'a> IntoBindGroupLayoutEntryBuilder for StorageTextureAtomic<'a> {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindingType::StorageTexture {
            access: StorageTextureAccess::Atomic,
            format: self.0.texture().format(),
            view_dimension: TextureViewDimension::D2,
        }
        .into_bind_group_layout_entry_builder()
    }
}
