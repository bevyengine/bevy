#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Provides 2D sprite rendering functionality.

extern crate alloc;

mod mesh2d;
mod render;
mod text2d;
mod texture_slice;
mod tilemap_chunk;

/// The sprite prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{ColorMaterial, MeshMaterial2d};
}

use bevy_shader::load_shader_library;
pub use mesh2d::*;
pub use render::*;
pub(crate) use texture_slice::*;
pub use tilemap_chunk::*;

use bevy_app::prelude::*;
use bevy_asset::{embedded_asset, AssetEventSystems};
use bevy_core_pipeline::core_2d::{AlphaMask2d, Opaque2d, Transparent2d};
use bevy_ecs::prelude::*;
use bevy_image::{prelude::*, TextureAtlasPlugin};
use bevy_mesh::Mesh2d;
use bevy_render::{
    batching::sort_binned_render_phase, render_phase::AddRenderCommand,
    render_resource::SpecializedRenderPipelines, sync_world::SyncToRenderWorld, ExtractSchedule,
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_sprite::Sprite;

use crate::text2d::extract_text2d_sprite;

/// Adds support for 2D sprite rendering.
#[derive(Default)]
pub struct SpriteRenderingPlugin;

/// System set for sprite rendering.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SpriteSystems {
    ExtractSprites,
    ComputeSlices,
}

/// Deprecated alias for [`SpriteSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `SpriteSystems`.")]
pub type SpriteSystem = SpriteSystems;

impl Plugin for SpriteRenderingPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "render/sprite_view_bindings.wgsl");

        embedded_asset!(app, "render/sprite.wgsl");

        if !app.is_plugin_added::<TextureAtlasPlugin>() {
            app.add_plugins(TextureAtlasPlugin);
        }

        app.add_plugins((
            Mesh2dRenderPlugin,
            ColorMaterialPlugin,
            TilemapChunkPlugin,
            TilemapChunkMaterialPlugin,
        ))
        .add_systems(
            PostUpdate,
            (
                compute_slices_on_asset_event.before(AssetEventSystems),
                compute_slices_on_sprite_change,
            )
                .in_set(SpriteSystems::ComputeSlices),
        );

        app.register_required_components::<Sprite, SyncToRenderWorld>();

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<SpecializedRenderPipelines<SpritePipeline>>()
                .init_resource::<SpriteMeta>()
                .init_resource::<ExtractedSprites>()
                .init_resource::<ExtractedSlices>()
                .init_resource::<SpriteAssetEvents>()
                .init_resource::<SpriteBatches>()
                .add_render_command::<Transparent2d, DrawSprite>()
                .add_systems(RenderStartup, init_sprite_pipeline)
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_sprites.in_set(SpriteSystems::ExtractSprites),
                        extract_sprite_events,
                        extract_text2d_sprite.after(SpriteSystems::ExtractSprites),
                    ),
                )
                .add_systems(
                    Render,
                    (
                        queue_sprites
                            .in_set(RenderSystems::Queue)
                            .ambiguous_with(queue_material2d_meshes::<ColorMaterial>),
                        prepare_sprite_image_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                        prepare_sprite_view_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                        sort_binned_render_phase::<Opaque2d>.in_set(RenderSystems::PhaseSort),
                        sort_binned_render_phase::<AlphaMask2d>.in_set(RenderSystems::PhaseSort),
                    ),
                );
        };
    }
}
