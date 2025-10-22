use crate::render_resource::{Buffer, TextureView};
use wgpu::{BindingResource, BindingType};

pub trait RenderTaskBindable {
    fn bind_info<'a>(&'a self) -> (BindingType, BindingResource<'a>);
}

pub struct StorageBuffer<'a>(pub &'a Buffer);

pub struct StorageBufferWriteable<'a>(pub &'a Buffer);

pub struct Texture<'a>(pub &'a TextureView);

pub struct StorageTexture<'a>(pub &'a TextureView);

pub struct StorageTextureWriteOnly<'a>(pub &'a TextureView);

pub struct StorageTextureReadOnly<'a>(pub &'a TextureView);
