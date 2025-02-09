//! This crate provides the tools for positioning and rendering text in Bevy.
//!
//! # `Font`
//!
//! Fonts contain information for drawing glyphs, which are shapes that typically represent a single character,
//! but in some cases part of a "character" (grapheme clusters) or more than one character (ligatures).
//!
//! A font *face* is part of a font family,
//! and is distinguished by its style (e.g. italic), its weight (e.g. bold) and its stretch (e.g. condensed).
//!
//! In Bevy, [`Font`]s are loaded by the [`FontLoader`] as [assets](bevy_asset::AssetPlugin).
//!
//! # `TextPipeline`
//!
//! The [`TextPipeline`] resource does all of the heavy lifting for rendering text.
//!
//! UI `Text` is first measured by creating a [`TextMeasureInfo`] in [`TextPipeline::create_text_measure`],
//! which is called by the `measure_text_system` system of `bevy_ui`.
//!
//! Note that text measurement is only relevant in a UI context.
//!
//! With the actual text bounds defined, the `bevy_ui::widget::text::text_system` system (in a UI context)
//! or [`text2d::update_text2d_layout`] system (in a 2d world space context)
//! passes it into [`TextPipeline::queue_text`], which:
//!
//! 1. updates a [`Buffer`](cosmic_text::Buffer) from the [`TextSpan`]s, generating new [`FontAtlasSet`]s if necessary.
//! 2. iterates over each glyph in the [`Buffer`](cosmic_text::Buffer) to create a [`PositionedGlyph`],
//!    retrieving glyphs from the cache, or rasterizing to a [`FontAtlas`] if necessary.
//! 3. [`PositionedGlyph`]s are stored in a [`TextLayoutInfo`],
//!    which contains all the information that downstream systems need for rendering.

extern crate alloc;

mod bounds;
mod error;
mod font;
mod font_atlas;
mod font_atlas_set;
mod font_loader;
mod glyph;
mod pipeline;
mod text;
mod text2d;
mod text_access;

pub use bounds::*;
pub use error::*;
pub use font::*;
pub use font_atlas::*;
pub use font_atlas_set::*;
pub use font_loader::*;
pub use glyph::*;
pub use pipeline::*;
pub use text::*;
pub use text2d::*;
pub use text_access::*;

/// The text prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        Font, JustifyText, LineBreak, Text2d, Text2dReader, Text2dWriter, TextColor, TextError,
        TextFont, TextLayout, TextSpan,
    };
}

use bevy_app::{prelude::*, Animation};
#[cfg(feature = "default_font")]
use bevy_asset::{load_internal_binary_asset, Handle};
use bevy_asset::{AssetApp, AssetEvents};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::CameraUpdateSystem, view::VisibilitySystems, ExtractSchedule, RenderApp,
};
use bevy_sprite::SpriteSystem;

/// The raw data for the default font used by `bevy_text`
#[cfg(feature = "default_font")]
pub const DEFAULT_FONT_DATA: &[u8] = include_bytes!("FiraMono-subset.ttf");

/// Adds text rendering support to an app.
///
/// When the `bevy_text` feature is enabled with the `bevy` crate, this
/// plugin is included by default in the `DefaultPlugins`.
#[derive(Default)]
pub struct TextPlugin;

/// Text is rendered for two different view projections;
/// 2-dimensional text ([`Text2d`]) is rendered in "world space" with a `BottomToTop` Y-axis,
/// while UI is rendered with a `TopToBottom` Y-axis.
/// This matters for text because the glyph positioning is different in either layout.
/// For `TopToBottom`, 0 is the top of the text, while for `BottomToTop` 0 is the bottom.
pub enum YAxisOrientation {
    /// Top to bottom Y-axis orientation, for UI
    TopToBottom,
    /// Bottom to top Y-axis orientation, for 2d world space
    BottomToTop,
}

/// System set in [`PostUpdate`] where all 2d text update systems are executed.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct Update2dText;

impl Plugin for TextPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Font>()
            .register_type::<Text2d>()
            .register_type::<TextFont>()
            .register_type::<LineHeight>()
            .register_type::<TextColor>()
            .register_type::<TextSpan>()
            .register_type::<TextBounds>()
            .register_type::<TextLayout>()
            .register_type::<ComputedTextBlock>()
            .register_type::<TextEntity>()
            .init_asset_loader::<FontLoader>()
            .init_resource::<FontAtlasSets>()
            .init_resource::<TextPipeline>()
            .init_resource::<CosmicFontSystem>()
            .init_resource::<SwashCache>()
            .init_resource::<TextIterScratch>()
            .add_systems(
                PostUpdate,
                (
                    remove_dropped_font_atlas_sets.before(AssetEvents),
                    detect_text_needs_rerender::<Text2d>,
                    update_text2d_layout
                        // Potential conflict: `Assets<Image>`
                        // In practice, they run independently since `bevy_render::camera_update_system`
                        // will only ever observe its own render target, and `update_text2d_layout`
                        // will never modify a pre-existing `Image` asset.
                        .ambiguous_with(CameraUpdateSystem),
                    calculate_bounds_text2d.in_set(VisibilitySystems::CalculateBounds),
                )
                    .chain()
                    .in_set(Update2dText)
                    .after(Animation),
            )
            .add_systems(Last, trim_cosmic_cache);

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
