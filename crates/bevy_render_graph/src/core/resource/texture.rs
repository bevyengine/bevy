use bevy_math::FloatOrd;
use std::borrow::Cow;
use std::hash::Hash;

use crate::core::{NodeContext, RenderGraphBuilder};

use bevy_render::render_resource::{
    FilterMode, Sampler, SamplerBindingType, SamplerDescriptor, Texture, TextureDescriptor,
    TextureUsages, TextureView, TextureViewDescriptor,
};

use super::{
    CacheRenderResource, IntoRenderResource, RenderHandle, RenderResource, ResourceType,
    UsagesRenderResource, WriteRenderResource,
};

impl RenderResource for Texture {
    const RESOURCE_TYPE: ResourceType = ResourceType::Texture;
    type Meta<'g> = TextureDescriptor<'static>;

    #[inline]
    fn import<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        meta: Self::Meta<'g>,
        resource: Cow<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.import_texture(meta, resource)
    }

    #[inline]
    fn get<'n, 'g: 'n>(
        context: &'n NodeContext<'n, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'n Self> {
        context.get_texture(resource)
    }

    #[inline]
    fn get_meta<'a, 'b: 'a, 'g: 'b>(
        graph: &'a RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Meta<'g>> {
        graph.get_texture_meta(resource)
    }
}

impl WriteRenderResource for Texture {}

impl UsagesRenderResource for Texture {
    type Usages = TextureUsages;

    #[inline]
    fn get_meta_mut<'a, 'b: 'a, 'g: 'b>(
        graph: &'a mut RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a mut Self::Meta<'g>> {
        graph.get_texture_meta_mut(resource)
    }

    #[inline]
    fn has_usages(meta: &Self::Meta<'_>, usages: &Self::Usages) -> bool {
        meta.usage.contains(*usages)
    }

    #[inline]
    fn add_usages(meta: &mut Self::Meta<'_>, usages: Self::Usages) {
        meta.usage.insert(usages);
    }
}

impl<'g> IntoRenderResource<'g> for TextureDescriptor<'static> {
    type Resource = Texture;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_texture(self)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RenderGraphTextureViewDescriptor<'g> {
    pub texture: RenderHandle<'g, Texture>,
    pub descriptor: TextureViewDescriptor<'static>,
}

impl<'g> Hash for RenderGraphTextureViewDescriptor<'g> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.texture.hash(state);
        let TextureViewDescriptor {
            label: _,
            format,
            dimension,
            aspect,
            base_mip_level,
            mip_level_count,
            base_array_layer,
            array_layer_count,
        } = &self.descriptor;
        format.hash(state);
        dimension.hash(state);
        aspect.hash(state);
        base_mip_level.hash(state);
        mip_level_count.hash(state);
        base_array_layer.hash(state);
        array_layer_count.hash(state);
    }
}

impl RenderResource for TextureView {
    const RESOURCE_TYPE: ResourceType = ResourceType::TextureView;
    type Meta<'g> = RenderGraphTextureViewDescriptor<'g>;

    #[inline]
    fn import<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        meta: Self::Meta<'g>,
        resource: Cow<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.import_texture_view(meta, resource)
    }

    #[inline]
    fn get<'n, 'g: 'n>(
        context: &'n NodeContext<'n, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'n Self> {
        context.get_texture_view(resource)
    }

    #[inline]
    fn get_meta<'a, 'b: 'a, 'g: 'b>(
        graph: &'a RenderGraphBuilder<'a, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Meta<'g>> {
        graph.get_texture_view_meta(resource)
    }
}

impl WriteRenderResource for TextureView {}

impl<'g> IntoRenderResource<'g> for RenderGraphTextureViewDescriptor<'g> {
    type Resource = TextureView;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_texture_view(self)
    }
}

impl RenderResource for Sampler {
    const RESOURCE_TYPE: ResourceType = ResourceType::Sampler;
    type Meta<'g> = RenderGraphSamplerDescriptor;

    #[inline]
    fn import<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        meta: Self::Meta<'g>,
        resource: Cow<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.import_sampler(meta, resource)
    }

    #[inline]
    fn get<'n, 'g: 'n>(
        context: &'n NodeContext<'n, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'n Self> {
        context.get_sampler(resource)
    }

    #[inline]
    fn get_meta<'a, 'b: 'a, 'g: 'b>(
        graph: &'a RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Meta<'g>> {
        graph.get_sampler_meta(resource)
    }
}

impl CacheRenderResource for Sampler {
    type Key = RenderGraphSamplerDescriptor;

    #[inline]
    fn key_from_meta<'b, 'g: 'b>(meta: &'b Self::Meta<'g>) -> &'b Self::Key {
        meta
    }
}

#[derive(Clone, Debug)]
pub struct RenderGraphSamplerDescriptor(pub SamplerDescriptor<'static>);

impl RenderGraphSamplerDescriptor {
    pub fn binding_type(&self) -> SamplerBindingType {
        if self.0.compare.is_some() {
            SamplerBindingType::Comparison
        } else if [self.0.min_filter, self.0.mag_filter, self.0.mipmap_filter]
            .iter()
            .any(|f| *f == FilterMode::Linear)
        {
            SamplerBindingType::Filtering
        } else {
            SamplerBindingType::NonFiltering
        }
    }
}

impl Hash for RenderGraphSamplerDescriptor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let SamplerDescriptor {
            label: _, //Note: labels aren't hashed, which might result in mismatched labels but otherwise should be fine
            address_mode_u,
            address_mode_v,
            address_mode_w,
            mag_filter,
            min_filter,
            mipmap_filter,
            lod_min_clamp,
            lod_max_clamp,
            compare,
            anisotropy_clamp,
            border_color,
        } = self.0;

        address_mode_u.hash(state);
        address_mode_v.hash(state);
        address_mode_w.hash(state);
        mag_filter.hash(state);
        min_filter.hash(state);
        mipmap_filter.hash(state);
        FloatOrd(lod_min_clamp).hash(state);
        FloatOrd(lod_max_clamp).hash(state);
        compare.hash(state);
        anisotropy_clamp.hash(state);
        border_color.hash(state);
    }
}

impl PartialEq for RenderGraphSamplerDescriptor {
    fn eq(&self, other: &Self) -> bool {
        let s = &self.0;
        let o = &other.0;

        s.address_mode_u == o.address_mode_u
            && s.address_mode_v == o.address_mode_v
            && s.address_mode_w == o.address_mode_w
            && s.mag_filter == o.mag_filter
            && s.min_filter == o.min_filter
            && s.mipmap_filter == o.mipmap_filter
            && FloatOrd(s.lod_min_clamp) == FloatOrd(o.lod_min_clamp)
            && FloatOrd(s.lod_max_clamp) == FloatOrd(o.lod_max_clamp)
            && s.compare == o.compare
            && s.anisotropy_clamp == o.anisotropy_clamp
            && s.border_color == o.border_color
    }
}

impl Eq for RenderGraphSamplerDescriptor {}

impl<'g> IntoRenderResource<'g> for RenderGraphSamplerDescriptor {
    type Resource = Sampler;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_sampler(self)
    }
}

impl<'g> IntoRenderResource<'g> for SamplerDescriptor<'static> {
    type Resource = Sampler;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_sampler(RenderGraphSamplerDescriptor(self))
    }
}
