use bevy_ecs::world::World;
use bevy_utils::define_label;
use wgpu::ImageDataLayout;

use crate::{
    render_graph::InternedRenderLabel,
    render_graph_v2::{RenderGraph, RenderGraphBuilder},
    render_resource::{Texture, TextureDescriptor},
    renderer::RenderDevice,
};

use super::{
    render_deps, IntoRenderResource, LastFrameRenderResource, RenderHandle, RenderResource,
    RenderResourceId, RenderResourceInit, RenderResourceMeta, WriteRenderResource,
};

define_label!(TextureLabel, TEXTURE_LABEL_INTERNER);

impl RenderResource for Texture {
    type Descriptor = TextureDescriptor<'static>;
    type Data = Texture;

    fn insert_data<'a>(
        graph: &mut RenderGraph,
        key: RenderResourceId,
        data: RenderResourceInit<Self>,
    ) {
        graph.textures.insert(key, data);
    }

    fn get_data<'a>(
        graph: &'a RenderGraph,
        _world: &'a World,
        key: RenderResourceId,
    ) -> Option<&'a RenderResourceMeta<Self>> {
        graph.textures.get_data(key)
    }

    fn from_data<'a>(data: &'a Self::Data, _world: &'a World) -> Option<&'a Self> {
        Some(data)
    }
}

impl WriteRenderResource for Texture {}

impl LastFrameRenderResource for Texture {
    fn send_next_frame(graph: &mut RenderGraph, key: RenderResourceId) {
        todo!()
    }

    fn get_last_frame(graph: &RenderGraph, label: InternedRenderLabel) -> RenderResourceMeta<Self> {
        todo!()
    }
}

impl IntoRenderResource for TextureDescriptor<'static> {
    type Resource = Texture;

    fn into_render_resource(
        self,
        _world: &World,
        render_device: &RenderDevice,
    ) -> RenderResourceInit<Self::Resource> {
        let tex = render_device.create_texture(&self);
        let meta = RenderResourceMeta {
            descriptor: Some(self),
            resource: tex,
        };
        RenderResourceInit::Eager(meta)
    }
}

pub fn new_texture_with_data(
    graph: &mut RenderGraphBuilder,
    descriptor: TextureDescriptor<'static>,
    data_layout: ImageDataLayout,
    data: &'static [u8],
) -> RenderHandle<Texture> {
    let size = descriptor.size;
    let mut tex = graph.new_resource(descriptor);
    graph.add_node(render_deps(&mut tex), move |ctx, _, queue| {
        queue.write_texture(ctx.get(tex).as_image_copy(), data, data_layout, size);
    });
    tex
}
