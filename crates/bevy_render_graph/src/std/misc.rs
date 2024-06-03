use bevy_render::render_resource::{
    Texture, TextureAspect, TextureFormat, TextureView, TextureViewDescriptor,
};

use crate::core::{
    resource::{RenderGraphTextureViewDescriptor, RenderHandle},
    RenderGraphBuilder,
};

///Returns the default texture view for a given texture
pub fn default_view<'g>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    texture: RenderHandle<'g, Texture>,
) -> RenderHandle<'g, TextureView> {
    let label = graph.meta(texture).label;
    graph.new_resource(RenderGraphTextureViewDescriptor {
        texture,
        descriptor: TextureViewDescriptor {
            label,
            format: None,
            dimension: None,
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        },
    })
}

///Returns the texture format of a texture view
pub fn texture_view_format<'g>(
    graph: &RenderGraphBuilder<'_, 'g>,
    texture_view: RenderHandle<'g, TextureView>,
) -> TextureFormat {
    let texture_view_meta = graph.meta(texture_view);
    texture_view_meta
        .descriptor
        .format
        .unwrap_or_else(|| graph.meta(texture_view_meta.texture).format)
}
