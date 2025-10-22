use crate::render_resource::{Buffer, IntoBinding, TextureView};
use wgpu::BindingResource;

pub struct StorageBuffer<'a>(pub &'a Buffer);

pub struct StorageBufferWriteable<'a>(pub &'a Buffer);

pub struct Texture<'a>(pub &'a TextureView);

pub struct StorageTexture<'a>(pub &'a TextureView);

pub struct StorageTextureWriteOnly<'a>(pub &'a TextureView);

pub struct StorageTextureReadOnly<'a>(pub &'a TextureView);

impl<'a> IntoBinding<'a> for StorageBuffer<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        self.0.as_entire_binding()
    }
}

impl<'a> IntoBinding<'a> for StorageBufferWriteable<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        self.0.as_entire_binding()
    }
}

impl<'a> IntoBinding<'a> for Texture<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        BindingResource::TextureView(self.0)
    }
}

impl<'a> IntoBinding<'a> for StorageTexture<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        BindingResource::TextureView(self.0)
    }
}

impl<'a> IntoBinding<'a> for StorageTextureWriteOnly<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        BindingResource::TextureView(self.0)
    }
}

impl<'a> IntoBinding<'a> for StorageTextureReadOnly<'a> {
    fn into_binding(self) -> BindingResource<'a> {
        BindingResource::TextureView(self.0)
    }
}
