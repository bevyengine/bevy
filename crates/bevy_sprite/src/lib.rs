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
        texture_slice::{BorderRect, SliceScaleMode, TextureSlice, TextureSlicer},
        ScalingMode,
    };
}

#[cfg(feature = "bevy_sprite_picking_backend")]
pub use picking_backend::*;
pub use sprite::*;
pub use texture_slice::*;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_image::TextureAtlasPlugin;

/// Adds support for 2D sprite.
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

        #[cfg(feature = "bevy_sprite_picking_backend")]
        app.add_plugins(SpritePickingPlugin);

    }
}
