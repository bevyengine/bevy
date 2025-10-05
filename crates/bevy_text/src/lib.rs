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
//! or `bevy_sprite::text2d::update_text2d_layout` system (in a 2d world space context)
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
pub use text_access::*;

pub use cosmic_text::{Stretch, Style, Weight};

/// The text prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        Family, Font, Justify, LineBreak, Stretch, Style, TextColor, TextError, TextFont,
        TextLayout, TextSpan, Weight,
    };
}

use bevy_app::prelude::*;
use bevy_asset::{AssetApp, AssetEventSystems};
use bevy_ecs::prelude::*;

/// The raw data for the default font used by `bevy_text`
#[cfg(feature = "default_font")]
pub const DEFAULT_FONT_DATA: &[u8] = include_bytes!("FiraMono-subset.ttf");

/// Adds text rendering support to an app.
///
/// When the `bevy_text` feature is enabled with the `bevy` crate, this
/// plugin is included by default in the `DefaultPlugins`.
pub struct TextPlugin {
    /// If `true`, the [`CosmicFontSystem`] will load system fonts.
    ///
    /// Supports Windows, Linux, and MacOS.
    ///
    /// See [`cosmic_text::fontdb::Database::load_system_fonts`] for details.
    pub load_system_fonts: bool,

    /// Override the family identifier for the system general Serif font
    pub family_serif: Option<String>,
    /// Override the default identifier for the general Sans-Serif font
    pub family_sans_serif: Option<String>,
    /// Override the default identifier for the general Cursive font
    pub family_cursive: Option<String>,
    /// Override the default identifier for the general Fantasy font
    pub family_fantasy: Option<String>,
    /// Override the default identifier for the general Monospace font
    pub family_monospace: Option<String>,
}
impl Default for TextPlugin {
    fn default() -> Self {
        Self {
            load_system_fonts: true,
            family_serif: None,
            family_sans_serif: None,
            family_cursive: None,
            family_fantasy: None,
            family_monospace: None,
        }
    }
}

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
            .insert_resource(CosmicFontSystem::new_with_settings(self))
            .init_resource::<SwashCache>()
            .init_resource::<TextIterScratch>()
            .add_systems(
                PostUpdate,
                remove_dropped_font_atlas_sets.before(AssetEventSystems),
            )
            .add_systems(Last, trim_cosmic_cache);

        #[cfg(feature = "default_font")]
        {
            use bevy_asset::{AssetId, Assets};
            let mut assets = app.world_mut().resource_mut::<Assets<_>>();
            let asset = Font::try_from_bytes(DEFAULT_FONT_DATA.to_vec()).unwrap();
            assets.insert(AssetId::default(), asset).unwrap();
        };
    }
}
