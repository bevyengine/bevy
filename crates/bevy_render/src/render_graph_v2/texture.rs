use wgpu::{ImageDataLayout, Texture, TextureDescriptor};

use crate::{
    renderer::RenderDevice,
    texture::{CachedTexture, TextureCache},
};

use super::{
    resource::{IntoRenderResource, RenderHandle, RenderResource},
    RenderGraphBuilder,
};

impl RenderResource for CachedTexture {}

impl IntoRenderResource for TextureDescriptor<'static> {
    type Resource = CachedTexture;

    fn into_render_resource(
        self,
        render_device: &RenderDevice,
        world: &bevy_ecs::world::World,
    ) -> Self::Resource {
        let mut texture_cache = world.resource_mut::<TextureCache>();
        texture_cache.get(render_device, self)
    }
}

pub fn new_texture_with_data(
    builder: &mut RenderGraphBuilder,
    descriptor: TextureDescriptor,
    data_layout: ImageDataLayout,
    data: &[u8],
) -> RenderHandle<CachedTexture> {
    let mut tex = builder.new_resource(descriptor);
    builder.add_node(&mut tex, move |ctx, device, queue| {
        queue.write_texture(
            ctx.get(&tex).texture.as_image_copy(),
            data,
            data_layout,
            descriptor.size,
        )
    });
    tex
}
