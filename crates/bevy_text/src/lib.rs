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

use bevy_camera::{visibility::VisibilitySystems, CameraUpdateSystems};
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
        Font, Justify, LineBreak, Text2d, Text2dReader, Text2dWriter, TextColor, TextError,
        TextFont, TextLayout, TextSpan,
    };
}

use bevy_app::{prelude::*, AnimationSystems};
use bevy_asset::{AssetApp, AssetEventSystems};
use bevy_ecs::prelude::*;
use bevy_render::{ExtractSchedule, RenderApp};
use bevy_sprite::SpriteSystems;

/// The raw data for the default font used by `bevy_text`
#[cfg(feature = "default_font")]
pub const DEFAULT_FONT_DATA: &[u8] = include_bytes!("FiraMono-subset.ttf");

/// Adds text rendering support to an app.
///
/// When the `bevy_text` feature is enabled with the `bevy` crate, this
/// plugin is included by default in the `DefaultPlugins`.
#[derive(Default)]
pub struct TextPlugin;

/// System set in [`PostUpdate`] where all 2d text update systems are executed.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct Text2dUpdateSystems;

/// Deprecated alias for [`Text2dUpdateSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `Text2dUpdateSystems`.")]
pub type Update2dText = Text2dUpdateSystems;

impl Plugin for TextPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Font>()
            .init_asset_loader::<FontLoader>()
            .init_resource::<FontAtlasSets>()
            .init_resource::<TextPipeline>()
            .init_resource::<CosmicFontSystem>()
            .init_resource::<SwashCache>()
            .init_resource::<TextIterScratch>()
            .add_systems(
                PostUpdate,
                (
                    remove_dropped_font_atlas_sets.before(AssetEventSystems),
                    detect_text_needs_rerender::<Text2d>,
                    update_text2d_layout
                        // Potential conflict: `Assets<Image>`
                        // In practice, they run independently since `bevy_render::camera_update_system`
                        // will only ever observe its own render target, and `update_text2d_layout`
                        // will never modify a pre-existing `Image` asset.
                        .ambiguous_with(CameraUpdateSystems),
                    calculate_bounds_text2d.in_set(VisibilitySystems::CalculateBounds),
                )
                    .chain()
                    .in_set(Text2dUpdateSystems)
                    .after(AnimationSystems),
            )
            .add_systems(Last, trim_cosmic_cache);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                ExtractSchedule,
                extract_text2d_sprite.after(SpriteSystems::ExtractSprites),
            );
        }

        #[cfg(feature = "default_font")]
        {
            use bevy_asset::{AssetId, Assets};
            let mut assets = app.world_mut().resource_mut::<Assets<_>>();
            let asset = Font::try_from_bytes(DEFAULT_FONT_DATA.to_vec()).unwrap();
            assets.insert(AssetId::default(), asset).unwrap();
        };
    }
}
