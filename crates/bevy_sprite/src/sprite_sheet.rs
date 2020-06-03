use crate::Rect;
use bevy_app::{Events, GetEventReader};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_core::bytes::AsBytes;
use bevy_derive::{Bytes, Uniform, Uniforms};
use bevy_render::{
    render_resource::{BufferInfo, BufferUsage, RenderResourceAssignment, ResourceInfo},
    renderer::{RenderResourceContext, RenderResources},
    texture::Texture,
    Renderable,
};
use glam::{Vec3, Vec4};
use legion::prelude::*;
use std::collections::HashSet;

#[derive(Uniforms)]
pub struct SpriteSheet {
    pub texture: Handle<Texture>,
    pub sprites: Vec<Rect>,
}

// NOTE: cannot do unsafe impl Byteable here because Vec3 takes up the space of a Vec4. If/when glam changes this we can swap out
// Bytes for Byteable. https://github.com/bitshifter/glam-rs/issues/36
#[derive(Uniform, Bytes, Default)]
pub struct SpriteSheetSprite {
    pub position: Vec3,
    pub index: u32,
}

pub const SPRITE_SHEET_BUFFER_ASSET_INDEX: usize = 0;

fn remove_sprite_sheet_resource(
    render_resources: &dyn RenderResourceContext,
    handle: Handle<SpriteSheet>,
) {
    if let Some(resource) =
        render_resources.get_asset_resource(handle, SPRITE_SHEET_BUFFER_ASSET_INDEX)
    {
        render_resources.remove_buffer(resource);
        render_resources.remove_asset_resource(handle, SPRITE_SHEET_BUFFER_ASSET_INDEX);
    }
}

pub fn sprite_sheet_resource_provider_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut sprite_sheet_event_reader = resources.get_event_reader::<AssetEvent<SpriteSheet>>();
    (move |world: &mut SubWorld,
           render_resources: Res<RenderResources>,
           sprite_sheets: Res<Assets<SpriteSheet>>,
           sprite_sheet_events: Res<Events<AssetEvent<SpriteSheet>>>,
           query: &mut Query<(Read<Handle<SpriteSheet>>, Write<Renderable>)>| {
        let render_resources = &*render_resources.context;
        let mut changed_sprite_sheets = HashSet::new();
        for event in sprite_sheet_event_reader.iter(&sprite_sheet_events) {
            match event {
                AssetEvent::Created { handle } => {
                    changed_sprite_sheets.insert(*handle);
                }
                AssetEvent::Modified { handle } => {
                    changed_sprite_sheets.insert(*handle);
                    remove_sprite_sheet_resource(render_resources, *handle);
                }
                AssetEvent::Removed { handle } => {
                    remove_sprite_sheet_resource(render_resources, *handle);
                    // if sprite sheet was modified and removed in the same update, ignore the modification
                    // events are ordered so future modification events are ok
                    changed_sprite_sheets.remove(handle);
                }
            }
        }

        for changed_sprite_sheet_handle in changed_sprite_sheets.iter() {
            if let Some(sprite_sheet) = sprite_sheets.get(changed_sprite_sheet_handle) {
                let sprite_sheet_bytes = sprite_sheet.sprites.as_slice().as_bytes();
                let sprite_sheet_buffer = render_resources.create_buffer_with_data(
                    BufferInfo {
                        buffer_usage: BufferUsage::STORAGE,
                        ..Default::default()
                    },
                    &sprite_sheet_bytes,
                );

                render_resources.set_asset_resource(
                    *changed_sprite_sheet_handle,
                    sprite_sheet_buffer,
                    SPRITE_SHEET_BUFFER_ASSET_INDEX,
                );
            }
        }

        // TODO: remove this when batching is implemented
        for (handle, mut renderable) in query.iter_mut(world) {
            if let Some(sprite_sheet_buffer) =
                render_resources.get_asset_resource(*handle, SPRITE_SHEET_BUFFER_ASSET_INDEX)
            {
                let mut buffer_size = None;
                render_resources.get_resource_info(sprite_sheet_buffer, &mut |info| {
                    if let Some(ResourceInfo::Buffer(BufferInfo { size, .. })) = info {
                        buffer_size = Some(*size as u64)
                    }
                });
                renderable.render_resource_assignments.set(
                    "SpriteSheet",
                    RenderResourceAssignment::Buffer {
                        dynamic_index: None,
                        range: 0..buffer_size.unwrap(),
                        resource: sprite_sheet_buffer,
                    },
                )
            }
        }
    })
    .system()
}
