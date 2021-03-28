use crate::{
    render_graph::{Node, ResourceSlots},
    renderer::{BufferInfo, BufferUsage, RenderContext},
    texture::{Texture, TextureDescriptor, TEXTURE_ASSET_INDEX},
};
use bevy_app::Events;
use bevy_asset::{AssetEvent, Assets};
use bevy_ecs::world::World;
use bevy_utils::HashSet;

pub struct TextureCopyNode {
    pub name: &'static str,
}

impl TextureCopyNode {
    pub fn new(name: &'static str) -> Self {
        TextureCopyNode { name }
    }
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
        let texture_event_reader = texture_events.get_reader(self.name);
        for event in texture_event_reader.iter() {
            match event {
                AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                    if let Some(texture) = textures.get(handle) {
                        if copied_textures.contains(&handle.id) {
                            continue;
                        }

                        let texture_descriptor: TextureDescriptor = texture.into();
                        let width = texture.size.width as usize;
                        let aligned_width =
                            render_context.resources().get_aligned_texture_size(width);
                        let format_size = texture.format.pixel_size();
                        let mut aligned_data = vec![
                            0;
                            format_size
                                * aligned_width
                                * texture.size.height as usize
                                * texture.size.depth as usize
                        ];
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
                            .get_asset_resource(handle, TEXTURE_ASSET_INDEX)
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

                        copied_textures.insert(&handle.id);
                    }
                }
                AssetEvent::Removed { .. } => {}
            }
        }
    }
}
