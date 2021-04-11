#[cfg(feature = "hdr")]
mod hdr_texture_loader;
mod image_texture_loader;
mod sampler_descriptor;
#[allow(clippy::module_inception)]
mod texture;
mod texture_descriptor;
mod texture_dimension;

pub(crate) mod image_texture_conversion;

#[cfg(feature = "hdr")]
pub use hdr_texture_loader::*;
pub use image_texture_loader::*;
pub use sampler_descriptor::*;
pub use texture::*;
pub use texture_descriptor::*;
pub use texture_dimension::*;

use crate::{
    render_command::RenderCommandQueue,
    render_resource::{BufferInfo, BufferUsage},
    renderer::{RenderResourceContext, RenderResources},
};
use bevy_app::{App, CoreStage, Plugin};
use bevy_asset::{AddAsset, AssetEvent, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_utils::HashSet;

pub struct TexturePlugin;

impl Plugin for TexturePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "png")]
        {
            app.init_asset_loader::<ImageTextureLoader>();
        }

        app.add_system_to_stage(CoreStage::PostUpdate, texture_resource_system.system())
            .add_asset::<Texture>();
    }
}

// TODO: remove old system
pub fn texture_resource_system(
    render_resource_context: Res<RenderResources>,
    mut render_command_queue: ResMut<RenderCommandQueue>,
    mut textures: ResMut<Assets<Texture>>,
    mut texture_events: EventReader<AssetEvent<Texture>>,
) {
    let render_resource_context = &**render_resource_context;
    let mut changed_textures = HashSet::default();
    for event in texture_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                changed_textures.insert(handle);
            }
            AssetEvent::Modified { handle } => {
                changed_textures.insert(handle);
                remove_current_texture_resources(render_resource_context, handle, &mut textures);
            }
            AssetEvent::Removed { handle } => {
                remove_current_texture_resources(render_resource_context, handle, &mut textures);
                // if texture was modified and removed in the same update, ignore the
                // modification events are ordered so future modification
                // events are ok
                changed_textures.remove(handle);
            }
        }
    }

    for texture_handle in changed_textures.iter() {
        if let Some(texture) = textures.get_mut(*texture_handle) {
            // TODO: this avoids creating new textures each frame because storing gpu data in the texture flags it as
            // modified. this prevents hot reloading and therefore can't be used in an actual impl.
            if texture.gpu_data.is_some() {
                continue;
            }
            // TODO: free old buffers / textures / samplers

            // TODO: using Into for TextureDescriptor is weird
            let texture_descriptor: TextureDescriptor = (&*texture).into();
            let texture_id = render_resource_context.create_texture(texture_descriptor);

            let sampler_id = render_resource_context.create_sampler(&texture.sampler);

            let width = texture.size.width as usize;
            let aligned_width = render_resource_context.get_aligned_texture_size(width);
            let format_size = texture.format.pixel_size();
            let mut aligned_data = vec![
                0;
                format_size
                    * aligned_width
                    * texture.size.height as usize
                    * texture.size.depth_or_array_layers as usize
            ];
            texture
                .data
                .chunks_exact(format_size * width)
                .enumerate()
                .for_each(|(index, row)| {
                    let offset = index * aligned_width * format_size;
                    aligned_data[offset..(offset + width * format_size)].copy_from_slice(row);
                });
            let staging_buffer_id = render_resource_context.create_buffer_with_data(
                BufferInfo {
                    buffer_usage: BufferUsage::COPY_SRC,
                    ..Default::default()
                },
                &aligned_data,
            );
            texture.gpu_data = Some(GpuData {
                texture_id,
                sampler_id,
            });

            render_command_queue.copy_buffer_to_texture(
                staging_buffer_id,
                0,
                (format_size * aligned_width) as u32,
                texture_id,
                [0, 0, 0],
                0,
                texture_descriptor.size,
            );
            render_command_queue.free_buffer(staging_buffer_id);
        }
    }
}

fn remove_current_texture_resources(
    render_resource_context: &dyn RenderResourceContext,
    handle: &Handle<Texture>,
    textures: &mut Assets<Texture>,
) {
    if let Some(gpu_data) = textures.get_mut(handle).and_then(|t| t.gpu_data.take()) {
        render_resource_context.remove_texture(gpu_data.texture_id);
        render_resource_context.remove_sampler(gpu_data.sampler_id);
    }
}
