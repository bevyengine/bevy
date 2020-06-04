use crate::{
    render_graph::{Node, ResourceSlots},
    render_resource::{BufferInfo, BufferUsage},
    renderer::RenderContext,
    texture::{Texture, TextureDescriptor, TEXTURE_ASSET_INDEX},
};
use bevy_app::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets};

use legion::prelude::*;

#[derive(Default)]
pub struct TextureCopyNode {
    pub texture_event_reader: EventReader<AssetEvent<Texture>>,
}

impl Node for TextureCopyNode {
    fn update(
        &mut self,
        _world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let texture_events = resources.get::<Events<AssetEvent<Texture>>>().unwrap();
        let textures = resources.get::<Assets<Texture>>().unwrap();
        for event in self.texture_event_reader.iter(&texture_events) {
            match event {
                AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                    if let Some(texture) = textures.get(&handle) {
                        let texture_descriptor: TextureDescriptor = texture.into();
                        let texture_buffer = render_context.resources().create_buffer_with_data(
                            BufferInfo {
                                buffer_usage: BufferUsage::COPY_SRC,
                                ..Default::default()
                            },
                            &texture.data,
                        );

                        let texture_resource = render_context
                            .resources()
                            .get_asset_resource(*handle, TEXTURE_ASSET_INDEX)
                            .unwrap();

                        // TODO: bytes_per_row could be incorrect for some texture formats
                        render_context.copy_buffer_to_texture(
                            texture_buffer,
                            0,
                            4 * texture.size.x() as u32,
                            texture_resource,
                            [0, 0, 0],
                            0,
                            0,
                            texture_descriptor.size.clone(),
                        );
                        render_context.resources().remove_buffer(texture_buffer);
                    }
                }
                AssetEvent::Removed { .. } => {}
            }
        }
    }
}
