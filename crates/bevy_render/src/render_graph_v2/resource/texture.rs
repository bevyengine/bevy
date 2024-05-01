use bevy_math::FloatOrd;
use std::hash::Hash;
use wgpu::TextureUsages;

use crate::{
    render_graph_v2::{NodeContext, RenderGraph, RenderGraphBuilder},
    render_resource::{Sampler, SamplerDescriptor, Texture, TextureDescriptor, TextureView},
};

use super::{
    ref_eq::RefEq, DescribedRenderResource, IntoRenderResource, NewRenderResource, RenderHandle,
    RenderResource, UsagesRenderResource, WriteRenderResource,
};

impl RenderResource for Texture {
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        todo!()
    }
}

impl WriteRenderResource for Texture {}

impl DescribedRenderResource for Texture {
    type Descriptor = TextureDescriptor<'static>;

    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Option<Self::Descriptor>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_descriptor<'g>(
        graph: &RenderGraph<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'g Self::Descriptor> {
        todo!()
    }
}

impl UsagesRenderResource for Texture {
    type Usages = TextureUsages;

    fn get_descriptor_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a mut Self::Descriptor> {
        todo!()
    }

    fn has_usages<'g>(descriptor: &Self::Descriptor, usages: &Self::Usages) -> bool {
        descriptor.usage.contains(*usages)
    }

    fn add_usages<'g>(descriptor: &mut Self::Descriptor, usages: Self::Usages) {
        descriptor.usage.insert(usages);
    }
}

impl<'g> IntoRenderResource<'g> for TextureDescriptor<'static> {
    type Resource = Texture;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_resource(NewRenderResource::FromDescriptor(self))
    }
}

impl RenderResource for TextureView {
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        todo!()
    }
}

impl WriteRenderResource for TextureView {}

impl RenderResource for Sampler {
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        todo!()
    }
}

impl DescribedRenderResource for Sampler {
    type Descriptor = RenderGraphSamplerDescriptor;

    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Option<Self::Descriptor>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_descriptor<'g>(
        graph: &RenderGraph<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'g Self::Descriptor> {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct RenderGraphSamplerDescriptor(pub SamplerDescriptor<'static>);

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

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_resource(NewRenderResource::FromDescriptor(self))
    }
}

impl<'g> IntoRenderResource<'g> for SamplerDescriptor<'static> {
    type Resource = Sampler;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_resource(NewRenderResource::FromDescriptor(
            RenderGraphSamplerDescriptor(self),
        ))
    }
}
