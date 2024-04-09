use bevy_ecs::world::World;
use wgpu::{ImageDataLayout, TextureDescriptor};

use crate::{render_resource::Texture, renderer::RenderDevice};

use super::{
    resource::{IntoRenderResource, RenderHandle, RenderResource},
    RenderGraphBuilder,
};

impl RenderResource for Texture {}

impl IntoRenderResource for TextureDescriptor<'static> {
    type Resource = Texture;

    fn into_render_resource(self, render_device: &RenderDevice, _world: &World) -> Self::Resource {
        render_device.create_texture(&self)
    }
}

pub fn new_texture_with_data(
    builder: &mut RenderGraphBuilder,
    descriptor: TextureDescriptor<'static>,
    data_layout: ImageDataLayout,
    data: &'static [u8],
) -> RenderHandle<Texture> {
    let mut tex = builder.new_resource(descriptor.clone());
    builder.add_node(&[tex.w()], move |ctx, _, queue| {
        queue.write_texture(
            ctx.get(tex).as_image_copy(),
            data,
            data_layout,
            descriptor.size,
        );
    });
    tex
}
