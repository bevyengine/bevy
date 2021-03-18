use crate::{
    render_graph::{Node, ResourceSlots},
    renderer::{BufferInfo, BufferUsage, RenderContext},
    texture::{Texture, TextureDescriptor, TEXTURE_ASSET_INDEX},
};
use bevy_app::{Events, ManualEventReader};
use bevy_asset::{AssetEvent, Assets};
use bevy_ecs::world::World;
use bevy_utils::HashSet;

#[derive(Default)]
pub struct TextureCopyNode {
    pub texture_event_reader: ManualEventReader<AssetEvent<Texture>>,
}

impl Node for TextureCopyNode {
    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let texture_events = world.get_resource::<Events<AssetEvent<Texture>>>().unwrap();
        let textures = world.get_resource::<Assets<Texture>>().unwrap();
        let mut copied_textures = HashSet::default();
        for event in self.texture_event_reader.iter(&texture_events) {
            match event {
                AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                    if let Some(texture) = textures.get(handle) {
                        if copied_textures.contains(&handle.id) {
                            continue;
                        }

                        let texture_descriptor: TextureDescriptor = texture.into();

                        debug_assert!(texture_descriptor.mip_level_count > 0);

                        for mip_level in 0..texture_descriptor.mip_level_count {
                            let mip_size = texture.mip_size(mip_level as usize);
                            let width = mip_size.width as usize;
                            let aligned_width =
                                render_context.resources().get_aligned_texture_size(width);
                            let format_size = texture.format.pixel_size();

                            let mut temporary_buffer;
                            let aligned_data = if aligned_width != width {
                                // must create a temporary buffer with the proper alignment
                                temporary_buffer = vec![
                                    0;
                                    format_size
                                        * aligned_width
                                        * mip_size.height as usize
                                        * mip_size.depth as usize
                                ];
                                texture.data[mip_level as usize]
                                    .chunks_exact(format_size * width)
                                    .enumerate()
                                    .for_each(|(index, row)| {
                                        let offset = index * aligned_width * format_size;
                                        temporary_buffer[offset..(offset + width * format_size)]
                                            .copy_from_slice(row);
                                    });
                                &temporary_buffer
                            } else {
                                // source data already has the correct alignment
                                &texture.data[mip_level as usize]
                            };

                            let texture_buffer =
                                render_context.resources().create_buffer_with_data(
                                    BufferInfo {
                                        buffer_usage: BufferUsage::COPY_SRC,
                                        ..Default::default()
                                    },
                                    &aligned_data,
                                );

                            let texture_resource = render_context
                                .resources()
                                .get_asset_resource(handle, TEXTURE_ASSET_INDEX)
                                .unwrap();

                            render_context.copy_buffer_to_texture(
                                texture_buffer,
                                0,
                                (format_size * aligned_width) as u32,
                                texture_resource.get_texture().unwrap(),
                                [0, 0, 0],
                                mip_level,
                                mip_size,
                            );
                            render_context.resources().remove_buffer(texture_buffer);
                        }

                        copied_textures.insert(&handle.id);
                    }
                }
                AssetEvent::Removed { .. } => {}
            }
        }
    }
}
