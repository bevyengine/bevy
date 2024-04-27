// FIXME(3492): remove once docs are ready
#![allow(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

mod error;
mod font;
mod font_atlas;
mod font_atlas_set;
mod font_loader;
mod glyph_brush;
mod pipeline;
mod text;
mod text2d;

pub use error::*;
pub use font::*;
pub use font_atlas::*;
pub use font_atlas_set::*;
pub use font_loader::*;
pub use glyph_brush::*;
pub use pipeline::*;
pub use text::*;
pub use text2d::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{Font, JustifyText, Text, Text2dBundle, TextError, TextSection, TextStyle};
}

use bevy_app::prelude::*;
use bevy_asset::AssetApp;
#[cfg(feature = "default_font")]
use bevy_asset::{load_internal_binary_asset, Handle};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::CameraUpdateSystem, view::VisibilitySystems, ExtractSchedule, RenderApp,
};
use bevy_sprite::SpriteSystem;
use std::num::NonZeroUsize;

/// Adds text rendering support to an app.
///
/// When the `bevy_text` feature is enabled with the `bevy` crate, this
/// plugin is included by default in the `DefaultPlugins`.
#[derive(Default)]
pub struct TextPlugin;

/// Settings used to configure the [`TextPlugin`].
#[derive(Resource)]
pub struct TextSettings {
    /// Soft maximum number of font atlases supported in a [`FontAtlasSet`]. When this is exceeded,
    /// a warning will be emitted a single time.
    pub soft_max_font_atlases: NonZeroUsize,
    /// Allows font size to be set dynamically exceeding the amount set in `soft_max_font_atlases`.
    /// Note each font size has to be generated which can have a strong performance impact.
    pub allow_dynamic_font_size: bool,
}

impl Default for TextSettings {
    fn default() -> Self {
        Self {
            soft_max_font_atlases: NonZeroUsize::new(16).unwrap(),
            allow_dynamic_font_size: false,
        }
    }
}

/// Text is rendered for two different view projections, a [`Text2dBundle`] is rendered with a
/// `BottomToTop` y axis, while UI is rendered with a `TopToBottom` y axis. This matters for text because
/// the glyph positioning is different in either layout.
pub enum YAxisOrientation {
    TopToBottom,
    BottomToTop,
}

/// A convenient alias for `With<Text>`, for use with
/// [`bevy_render::view::VisibleEntities`].
pub type WithText = With<Text>;

impl Plugin for TextPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Font>()
            .register_type::<Text>()
            .register_type::<Text2dBounds>()
            .init_asset_loader::<FontLoader>()
            .init_resource::<TextSettings>()
            .init_resource::<FontAtlasSets>()
            .insert_resource(TextPipeline::default())
            .add_systems(
                PostUpdate,
                (
                    calculate_bounds_text2d
                        .in_set(VisibilitySystems::CalculateBounds)
                        .after(update_text2d_layout),
                    update_text2d_layout
                        .after(font_atlas_set::remove_dropped_font_atlas_sets)
                        // Potential conflict: `Assets<Image>`
                        // In practice, they run independently since `bevy_render::camera_update_system`
                        // will only ever observe its own render target, and `update_text2d_layout`
                        // will never modify a pre-existing `Image` asset.
                        .ambiguous_with(CameraUpdateSystem),
                    remove_dropped_font_atlas_sets,
                ),
            );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                ExtractSchedule,
                extract_text2d_sprite.after(SpriteSystem::ExtractSprites),
            );
        }

        #[cfg(feature = "default_font")]
        load_internal_binary_asset!(
            app,
            Handle::default(),
            "FiraMono-subset.ttf",
            |bytes: &[u8], _path: String| { Font::try_from_bytes(bytes.to_vec()).unwrap() }
        );
    }
}
