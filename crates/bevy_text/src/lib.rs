#![allow(clippy::type_complexity)]

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
    pub use crate::{Font, Text, Text2dBundle, TextAlignment, TextError, TextSection, TextStyle};
}

use bevy_app::prelude::*;
#[cfg(feature = "default_font")]
use bevy_asset::load_internal_binary_asset;
use bevy_asset::{AssetApp, Handle};
use bevy_ecs::prelude::*;
use bevy_render::{camera::CameraUpdateSystem, ExtractSchedule, RenderApp};
use bevy_sprite::SpriteSystem;
use std::num::NonZeroUsize;

#[derive(Default)]
pub struct TextPlugin;

/// [`TextPlugin`] settings
#[derive(Resource)]
pub struct TextSettings {
    /// Maximum number of font atlases supported in a [`FontAtlasSet`].
    pub max_font_atlases: NonZeroUsize,
    /// Allows font size to be set dynamically exceeding the amount set in `max_font_atlases`.
    /// Note each font size has to be generated which can have a strong performance impact.
    pub allow_dynamic_font_size: bool,
}

impl Default for TextSettings {
    fn default() -> Self {
        Self {
            max_font_atlases: NonZeroUsize::new(16).unwrap(),
            allow_dynamic_font_size: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct FontAtlasWarning {
    warned: bool,
}

/// Text is rendered for two different view projections, normal `Text2DBundle` is rendered with a
/// `BottomToTop` y axis, and UI is rendered with a `TopToBottom` y axis. This matters for text because
/// the glyph positioning is different in either layout.
pub enum YAxisOrientation {
    TopToBottom,
    BottomToTop,
}

impl Plugin for TextPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Font>()
            .register_type::<Text>()
            .register_type::<Text2dBounds>()
            .register_type::<TextSection>()
            .register_type::<Vec<TextSection>>()
            .register_type::<TextStyle>()
            .register_type::<TextAlignment>()
            .register_type::<BreakLineOn>()
            .init_asset_loader::<FontLoader>()
            .init_resource::<TextSettings>()
            .init_resource::<FontAtlasWarning>()
            .init_resource::<FontAtlasSets>()
            .insert_resource(TextPipeline::default())
            .add_systems(
                PostUpdate,
                (
                    update_text2d_layout
                        // Potential conflict: `Assets<Image>`
                        // In practice, they run independently since `bevy_render::camera_update_system`
                        // will only ever observe its own render target, and `update_text2d_layout`
                        // will never modify a pre-existing `Image` asset.
                        .ambiguous_with(CameraUpdateSystem),
                    font_atlas_set::remove_dropped_font_atlas_sets,
                ),
            );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
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
