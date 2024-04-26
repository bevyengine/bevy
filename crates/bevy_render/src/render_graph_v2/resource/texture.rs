use bevy_ecs::world::World;
use bevy_math::FloatOrd;
use std::{borrow::Cow, hash::Hash};

use crate::{
    render_graph_v2::{seal, RenderGraph, RenderGraphBuilder},
    render_resource::{
        ImageDataLayout, Sampler, SamplerDescriptor, Texture, TextureDescriptor, TextureView,
        TextureViewDescriptor,
    },
    renderer::RenderDevice,
};

use super::{
    render_deps, CachedRenderStore, IntoRenderResource, RenderDependencies, RenderHandle,
    RenderResource, RenderResourceInit, RenderResourceMeta, SimpleRenderStore, WriteRenderResource,
};

impl seal::Super for Texture {}

impl RenderResource for Texture {
    type Descriptor = TextureDescriptor<'static>;
    type Data = Self;
    type Store<'g> = SimpleRenderStore<'g, Self>;

    fn get_store<'a, 'g: 'a>(graph: &'a RenderGraph<'g>, _: seal::Token) -> &'a Self::Store<'g> {
        todo!()
    }

    fn get_store_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        _: seal::Token,
    ) -> &'a mut Self::Store<'g> {
        todo!()
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        todo!()
    }

    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        render_device: &RenderDevice,
    ) -> Self::Data {
        todo!()
    }
}

impl WriteRenderResource for Texture {}

// impl RetainedRenderResource for Texture {}

impl<'g> IntoRenderResource<'g> for TextureDescriptor<'static> {
    type Resource = Texture;

    fn into_render_resource(
        self,
        _world: &World,
        render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource> {
        let tex = render_device.create_texture(&self);
        let meta = RenderResourceMeta {
            descriptor: Some(self),
            resource: Cow::Owned(tex),
        };
        RenderResourceInit::Resource(meta)
    }
}

pub fn new_texture_with_data<'g>(
    graph: &mut RenderGraphBuilder<'g>,
    descriptor: TextureDescriptor<'static>,
    data_layout: ImageDataLayout,
    data: &'static [u8],
) -> RenderHandle<'g, Texture> {
    let size = descriptor.size;
    let mut tex = graph.new_resource(descriptor);
    graph.add_node(render_deps(&mut tex), move |ctx, _, queue| {
        //todo: internal mutability on ctx?
        queue.write_texture(ctx.get(tex).as_image_copy(), data, data_layout, size);
    });
    tex
}

impl seal::Super for TextureView {}

impl RenderResource for TextureView {
    type Descriptor = TextureViewDescriptor<'static>;
    type Data = Self;
    type Store<'g> = SimpleRenderStore<'g, TextureView>;

    fn get_store<'a, 'g: 'a>(graph: &'a RenderGraph<'g>, _: seal::Token) -> &'a Self::Store<'g> {
        todo!()
    }

    fn get_store_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        _: seal::Token,
    ) -> &'a mut Self::Store<'g> {
        todo!()
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        todo!()
    }

    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        render_device: &RenderDevice,
    ) -> Self::Data {
        todo!()
    }
}

impl seal::Super for Sampler {}

impl RenderResource for Sampler {
    type Descriptor = RenderGraphSamplerDescriptor;
    type Data = Self;
    type Store<'g> = CachedRenderStore<'g, Self>;

    fn get_store<'a, 'g: 'a>(graph: &'a RenderGraph<'g>, _: seal::Token) -> &'a Self::Store<'g> {
        todo!()
    }

    fn get_store_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        _: seal::Token,
    ) -> &'a mut Self::Store<'g> {
        todo!()
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        todo!()
    }

    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        render_device: &RenderDevice,
    ) -> Self::Data {
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
        world: &World,
        render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource> {
        RenderResourceInit::FromDescriptor(self)
    }
}

impl<'g> IntoRenderResource<'g> for SamplerDescriptor<'static> {
    type Resource = Sampler;

    fn into_render_resource(
        self,
        world: &World,
        render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource> {
        RenderResourceInit::FromDescriptor(RenderGraphSamplerDescriptor(self))
    }
}
