//! This module exports types related to rendering glyphs.

use bevy_asset::Handle;
use bevy_math::{IVec2, Vec2};
use bevy_sprite::TextureAtlas;

/// A glyph of a font, typically representing a single character, positioned in screen space.
///
/// Contains information about how and where to render a glyph.
///
/// Used in [`TextPipeline::queue_text`](crate::TextPipeline::queue_text) and [`crate::TextLayoutInfo`] for rendering glyphs.
#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    /// The position of the glyph in the [`Text`](crate::Text)'s bounding box.
    pub position: Vec2,
    /// The width and height of the glyph in logical pixels.
    pub size: Vec2,
    /// Information about the glyph's atlas.
    pub atlas_info: GlyphAtlasInfo,
    /// The index of the glyph in the [`Text`](crate::Text)'s sections.
    pub section_index: usize,
    /// In order to do text editing, we need access to the size of glyphs and their index in the associated String.
    /// For example, to figure out where to place the cursor in an input box from the mouse's position.
    /// Without this, it's only possible in texts where each glyph is one byte.
    // TODO: re-implement this or equivalent
    pub byte_index: usize,
}

/// Information about a glyph in an atlas.
///
/// Rasterized glyphs are stored as rectangles
/// in one or more [`FontAtlas`](crate::FontAtlas)es.
///
/// Used in [`PositionedGlyph`] and [`FontAtlasSet`](crate::FontAtlasSet).
#[derive(Debug, Clone)]
pub struct GlyphAtlasInfo {
    /// A handle to the texture atlas this glyph was placed in.
    pub texture_atlas: Handle<TextureAtlas>,
    /// Location and offset of a glyph.
    pub location: GlyphAtlasLocation,
}

/// The location of a glyph in an atlas,
/// and how it should be positioned when placed.
///
/// Used in [`GlyphAtlasInfo`] and [`FontAtlas`](crate::FontAtlas).
#[derive(Debug, Clone, Copy)]
pub struct GlyphAtlasLocation {
    /// The index of the glyph in the atlas
    pub glyph_index: usize,
    /// The required offset (relative positioning) when placed
    pub offset: IVec2,
}
