use crate::render_resource::{
    BindGroupLayoutEntryBuilder, Buffer, IntoBindGroupLayoutEntryBuilder, IntoBinding, TextureView,
};
use wgpu::{BindingResource, BindingType, BufferBindingType, StorageTextureAccess};

#[derive(Clone)]
pub struct StorageBuffer<'a>(pub &'a Buffer);

#[derive(Clone)]
pub struct StorageBufferWriteable<'a>(pub &'a Buffer);

#[derive(Clone)]
pub struct Texture<'a>(pub &'a TextureView);

#[derive(Clone)]
pub struct StorageTexture<'a>(pub &'a TextureView);

#[derive(Clone)]
pub struct StorageTextureWriteOnly<'a>(pub &'a TextureView);

#[derive(Clone)]
pub struct StorageTextureReadOnly<'a>(pub &'a TextureView);

#[derive(Clone)]
pub struct StorageTextureAtomic<'a>(pub &'a TextureView);

impl<'a> IntoBinding<'a> for StorageBuffer<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        self.0.as_entire_binding()
    }
}

impl<'a> IntoBindGroupLayoutEntryBuilder for StorageBuffer<'a> {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
        .into_bind_group_layout_entry_builder()
    }
}

impl<'a> IntoBinding<'a> for StorageBufferWriteable<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        self.0.as_entire_binding()
    }
}

impl<'a> IntoBindGroupLayoutEntryBuilder for StorageBufferWriteable<'a> {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
        .into_bind_group_layout_entry_builder()
    }
}

impl<'a> IntoBinding<'a> for Texture<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        BindingResource::TextureView(self.0)
    }
}

impl<'a> IntoBindGroupLayoutEntryBuilder for Texture<'a> {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindingType::Texture {
            sample_type: todo!(),
            view_dimension: todo!(),
            multisampled: self.0.texture().sample_count() > 1,
        }
        .into_bind_group_layout_entry_builder()
    }
}

impl<'a> IntoBinding<'a> for StorageTexture<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        BindingResource::TextureView(self.0)
    }
}

impl<'a> IntoBindGroupLayoutEntryBuilder for StorageTexture<'a> {
    fn into_bind_group_layout_entry_builder(self) -> BindGroupLayoutEntryBuilder {
        BindingType::StorageTexture {
            access: StorageTextureAccess::ReadWrite,
            format: self.0.texture().format(),
            view_dimension: todo!(),
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
            view_dimension: todo!(),
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
            view_dimension: todo!(),
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
            view_dimension: todo!(),
        }
        .into_bind_group_layout_entry_builder()
    }
}
