#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Provides 2D sprite functionality.

extern crate alloc;

#[cfg(feature = "bevy_sprite_picking_backend")]
mod picking_backend;
mod sprite;
mod text2d;
mod texture_slice;

/// The sprite prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[cfg(feature = "bevy_sprite_picking_backend")]
    #[doc(hidden)]
    pub use crate::picking_backend::{
        SpritePickingCamera, SpritePickingMode, SpritePickingPlugin, SpritePickingSettings,
    };
    #[doc(hidden)]
    pub use crate::{
        sprite::{Sprite, SpriteImageMode},
        text2d::{Text2d, Text2dReader, Text2dWriter},
        texture_slice::{BorderRect, SliceScaleMode, TextureSlice, TextureSlicer},
        ScalingMode,
    };
}

use bevy_app::AnimationSystems;
use bevy_camera::visibility::VisibilitySystems;
use bevy_camera::CameraUpdateSystems;
use bevy_text::detect_text_needs_rerender;
use bevy_text::Text2dUpdateSystems;
#[cfg(feature = "bevy_sprite_picking_backend")]
pub use picking_backend::*;
pub use sprite::*;
pub use text2d::*;
pub use texture_slice::*;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_image::TextureAtlasPlugin;

/// Adds support for 2D sprites.
#[derive(Default)]
pub struct SpritePlugin;

/// System set for sprite rendering.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SpriteSystems {
    ExtractSprites,
    ComputeSlices,
}

/// Deprecated alias for [`SpriteSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `SpriteSystems`.")]
pub type SpriteSystem = SpriteSystems;

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TextureAtlasPlugin>() {
            app.add_plugins(TextureAtlasPlugin);
        }

        app.add_systems(
            PostUpdate,
            (
                detect_text_needs_rerender::<Text2d>,
                update_text2d_layout
                    // Potential conflict: `Assets<Image>`
                    // In practice, they run independently since `bevy_render::camera_update_system`
                    // will only ever observe its own render target, and `update_text2d_layout`
                    // will never modify a pre-existing `Image` asset.
                    .ambiguous_with(CameraUpdateSystems)
                    .after(bevy_text::remove_dropped_font_atlas_sets),
                calculate_bounds_text2d.in_set(VisibilitySystems::CalculateBounds),
            )
                .chain()
                .in_set(Text2dUpdateSystems)
                .after(AnimationSystems),
        );

        #[cfg(feature = "bevy_sprite_picking_backend")]
        app.add_plugins(SpritePickingPlugin);
    }
}
