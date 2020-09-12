use super::{BufferId, SamplerId, TextureId};
use crate::texture::Texture;
use bevy_asset::Handle;

use bevy_core::{Byteable, Bytes};
pub use bevy_derive::{RenderResource, RenderResources};
use bevy_math::{Mat4, Vec2, Vec3, Vec4};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RenderResourceType {
    Buffer,
    Texture,
    Sampler,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum RenderResourceId {
    Buffer(BufferId),
    Texture(TextureId),
    Sampler(SamplerId),
}

impl From<BufferId> for RenderResourceId {
    fn from(value: BufferId) -> Self {
        RenderResourceId::Buffer(value)
    }
}

impl From<TextureId> for RenderResourceId {
    fn from(value: TextureId) -> Self {
        RenderResourceId::Texture(value)
    }
}

impl From<SamplerId> for RenderResourceId {
    fn from(value: SamplerId) -> Self {
        RenderResourceId::Sampler(value)
    }
}

impl RenderResourceId {
    pub fn get_texture(&self) -> Option<TextureId> {
        if let RenderResourceId::Texture(id) = self {
            Some(*id)
        } else {
            None
        }
    }

    pub fn get_buffer(&self) -> Option<BufferId> {
        if let RenderResourceId::Buffer(id) = self {
            Some(*id)
        } else {
            None
        }
    }

    pub fn get_sampler(&self) -> Option<SamplerId> {
        if let RenderResourceId::Sampler(id) = self {
            Some(*id)
        } else {
            None
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct RenderResourceHints: u32 {
        const BUFFER = 1;
    }
}

pub trait RenderResource {
    fn resource_type(&self) -> Option<RenderResourceType>;
    fn write_buffer_bytes(&self, buffer: &mut [u8]);
    fn buffer_byte_len(&self) -> Option<usize>;
    // TODO: consider making these panic by default, but return non-options
    fn texture(&self) -> Option<Handle<Texture>>;
}

pub trait RenderResources: Send + Sync + 'static {
    fn render_resources_len(&self) -> usize;
    fn get_render_resource(&self, index: usize) -> Option<&dyn RenderResource>;
    fn get_render_resource_name(&self, index: usize) -> Option<&str>;
    fn get_render_resource_hints(&self, _index: usize) -> Option<RenderResourceHints> {
        None
    }
    fn iter(&self) -> RenderResourceIterator;
}

pub struct RenderResourceIterator<'a> {
    render_resources: &'a dyn RenderResources,
    index: usize,
}

impl<'a> RenderResourceIterator<'a> {
    pub fn new(render_resources: &'a dyn RenderResources) -> Self {
        Self {
            render_resources,
            index: 0,
        }
    }
}
impl<'a> Iterator for RenderResourceIterator<'a> {
    type Item = &'a dyn RenderResource;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.render_resources.render_resources_len() {
            None
        } else {
            let render_resource = self
                .render_resources
                .get_render_resource(self.index)
                .unwrap();
            self.index += 1;
            Some(render_resource)
        }
    }
}

#[macro_export]
macro_rules! impl_render_resource_bytes {
    ($ty:ident) => {
        impl RenderResource for $ty {
            fn resource_type(&self) -> Option<RenderResourceType> {
                Some(RenderResourceType::Buffer)
            }

            fn write_buffer_bytes(&self, buffer: &mut [u8]) {
                self.write_bytes(buffer);
            }

            fn buffer_byte_len(&self) -> Option<usize> {
                Some(self.byte_len())
            }

            fn texture(&self) -> Option<Handle<Texture>> {
                None
            }
        }
    };
}

// TODO: when specialization lands, replace these with impl<T> RenderResource for T where T: Bytes
impl_render_resource_bytes!(Vec2);
impl_render_resource_bytes!(Vec3);
impl_render_resource_bytes!(Vec4);
impl_render_resource_bytes!(Mat4);
impl_render_resource_bytes!(u8);
impl_render_resource_bytes!(u16);
impl_render_resource_bytes!(u32);
impl_render_resource_bytes!(u64);
impl_render_resource_bytes!(i8);
impl_render_resource_bytes!(i16);
impl_render_resource_bytes!(i32);
impl_render_resource_bytes!(i64);
impl_render_resource_bytes!(f32);
impl_render_resource_bytes!(f64);

impl<T> RenderResource for Vec<T>
where
    T: Sized + Byteable,
{
    fn resource_type(&self) -> Option<RenderResourceType> {
        Some(RenderResourceType::Buffer)
    }

    fn write_buffer_bytes(&self, buffer: &mut [u8]) {
        self.write_bytes(buffer);
    }

    fn buffer_byte_len(&self) -> Option<usize> {
        Some(self.byte_len())
    }

    fn texture(&self) -> Option<Handle<Texture>> {
        None
    }
}

impl RenderResources for bevy_transform::prelude::GlobalTransform {
    fn render_resources_len(&self) -> usize {
        1
    }

    fn get_render_resource(&self, index: usize) -> Option<&dyn RenderResource> {
        if index == 0 {
            Some(self.value())
        } else {
            None
        }
    }

    fn get_render_resource_name(&self, index: usize) -> Option<&str> {
        if index == 0 {
            Some("Transform")
        } else {
            None
        }
    }

    fn iter(&self) -> RenderResourceIterator {
        RenderResourceIterator::new(self)
    }
}
