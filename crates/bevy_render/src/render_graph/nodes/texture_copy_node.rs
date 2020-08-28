use crate::{
    render_graph::{Node, ResourceSlots},
    renderer::{BufferInfo, BufferUsage, RenderContext},
    texture::{Texture, TextureDescriptor, TEXTURE_ASSET_INDEX},
};
use bevy_app::prelude::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets};
use bevy_ecs::{Resources, World};

#[derive(Default)]
pub struct TextureCopyNode {
    pub texture_event_reader: EventReader<AssetEvent<Texture>>,
}

pub const ALIGNMENT: usize = 256;
fn get_aligned(data_size: f32) -> usize {
    ALIGNMENT * ((data_size / ALIGNMENT as f32).ceil() as usize)
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
                        let width = texture.size.x() as usize;
                        let aligned_width = get_aligned(texture.size.x());
                        let format_size = texture.format.pixel_size();
                        let mut aligned_data =
                            vec![0; format_size * aligned_width * texture.size.y() as usize];
                        texture
                            .data
                            .chunks_exact(format_size * width)
                            .enumerate()
                            .for_each(|(index, row)| {
                                let offset = index * aligned_width * format_size;
                                aligned_data[offset..(offset + width * format_size)]
                                    .copy_from_slice(row);
                            });
                        let texture_buffer = render_context.resources().create_buffer_with_data(
                            BufferInfo {
                                buffer_usage: BufferUsage::COPY_SRC,
                                ..Default::default()
                            },
                            &aligned_data,
                        );

                        let texture_resource = render_context
                            .resources()
                            .get_asset_resource(*handle, TEXTURE_ASSET_INDEX)
                            .unwrap();

                        render_context.copy_buffer_to_texture(
                            texture_buffer,
                            0,
                            (format_size * aligned_width) as u32,
                            texture_resource.get_texture().unwrap(),
                            [0, 0, 0],
                            0,
                            texture_descriptor.size,
                        );
                        render_context.resources().remove_buffer(texture_buffer);
                    }
                }
                AssetEvent::Removed { .. } => {}
            }
        }
    }
}
