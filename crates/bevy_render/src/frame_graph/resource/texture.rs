use std::{borrow::Cow, sync::Arc};

use wgpu::SurfaceTexture;

use super::{
    AnyTransientResource, AnyFrameGraphResourceDescriptor, TransientResourceDescriptor,
    IntoArcTransientResource, ArcTransientResource, TransientResource,
};

pub struct TransientTexture {
    pub resource: wgpu::Texture,
    pub desc: TextureInfo,
}

impl IntoArcTransientResource for TransientTexture {
    fn into_arc_transient_resource(self: Arc<Self>) -> ArcTransientResource {
        ArcTransientResource::Texture(self)
    }
}

impl TransientResource for TransientTexture {
    type Descriptor = TextureInfo;

    fn borrow_resource(res: &AnyTransientResource) -> &Self {
        match res {
            AnyTransientResource::OwnedTexture(res) => res,
            AnyTransientResource::ImportedTexture(res) => res,
            _ => {
                unimplemented!()
            }
        }
    }

    fn get_desc(&self) -> &Self::Descriptor {
        &self.desc
    }
}

impl TransientTexture {
    pub fn new_arc_with_surface(surface: &SurfaceTexture) -> Arc<TransientTexture> {
        Arc::new(TransientTexture {
            desc: TextureInfo {
                label: None,
                size: wgpu::Extent3d {
                    width: surface.texture.width(),
                    height: surface.texture.height(),
                    depth_or_array_layers: surface.texture.depth_or_array_layers(),
                },
                mip_level_count: surface.texture.mip_level_count(),
                sample_count: surface.texture.sample_count(),
                dimension: surface.texture.dimension(),
                format: surface.texture.format(),
                usage: surface.texture.usage(),
                view_formats: vec![],
            },
            resource: surface.texture.clone(),
        })
    }
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct TextureInfo {
    pub label: Option<Cow<'static, str>>,
    pub size: wgpu::Extent3d,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: wgpu::TextureDimension,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub view_formats: Vec<wgpu::TextureFormat>,
}

impl From<TextureInfo> for AnyFrameGraphResourceDescriptor {
    fn from(value: TextureInfo) -> Self {
        AnyFrameGraphResourceDescriptor::Texture(value)
    }
}

impl TransientResourceDescriptor for TextureInfo {
    type Resource = TransientTexture;
}

impl TextureInfo {
    pub fn from_texture_desc(desc: &wgpu::TextureDescriptor) -> Self {
        TextureInfo {
            label: desc.label.map(|label| label.to_string().into()),
            size: desc.size,
            mip_level_count: desc.mip_level_count,
            sample_count: desc.sample_count,
            dimension: desc.dimension,
            format: desc.format,
            usage: desc.usage,
            view_formats: desc.view_formats.to_vec(),
        }
    }

    pub fn get_texture_desc(&self) -> wgpu::TextureDescriptor {
        wgpu::TextureDescriptor {
            label: self.label.as_deref(),
            size: self.size,
            mip_level_count: self.mip_level_count,
            sample_count: self.sample_count,
            dimension: self.dimension,
            format: self.format,
            usage: self.usage,
            view_formats: &self.view_formats,
        }
    }
}
