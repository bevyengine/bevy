//! This module exports types related to rendering glyphs.

use bevy_asset::AssetId;
use bevy_image::prelude::*;
use bevy_math::{Rect, Vec2};
use bevy_reflect::Reflect;

/// A glyph of a font, typically representing a single character, positioned in screen space.
///
/// Contains information about how and where to render a glyph.
///
/// Used in [`TextPipeline::update_text_layout_info`](crate::TextPipeline::update_text_layout_info) and [`TextLayoutInfo`](`crate::TextLayoutInfo`) for rendering glyphs.
#[derive(Debug, Clone, Reflect)]
#[reflect(Clone)]
pub struct PositionedGlyph {
    /// The position of the glyph in the text block's bounding box.
    pub position: Vec2,
    /// Information about the glyph's atlas.
    pub atlas_info: GlyphAtlasInfo,
    /// The index of the glyph in the [`ComputedTextBlock`](crate::ComputedTextBlock)'s tracked spans.
    pub span_index: usize,
    /// The index of the glyph's line.
    pub line_index: usize,
    /// The byte index of the glyph in its line.
    pub byte_index: usize,
    /// The byte length of the glyph.
    pub byte_length: usize,
}

/// Information about a glyph in an atlas.
///
/// Rasterized glyphs are stored as rectangles
/// in one or more [`FontAtlas`](crate::FontAtlas)es.
///
/// Used in [`PositionedGlyph`] and [`FontAtlasSet`](crate::FontAtlasSet).
#[derive(Debug, Clone, Reflect)]
#[reflect(Clone)]
pub struct GlyphAtlasInfo {
    /// An asset ID to the [`Image`] data for the texture atlas this glyph was placed in.
    ///
    /// An asset ID of the handle held by the [`FontAtlas`](crate::FontAtlas).
    pub texture: AssetId<Image>,
    /// Bounds of the glyph in the atlas texture
    pub rect: Rect,
    /// The required offset (relative positioning) when placed
    pub offset: Vec2,
}

/// The location of a glyph in an atlas,
/// and how it should be positioned when placed.
///
/// Used in [`GlyphAtlasInfo`] and [`FontAtlas`](crate::FontAtlas).
#[derive(Debug, Clone, Copy, Reflect)]
#[reflect(Clone)]
pub struct GlyphAtlasLocation {
    /// The index of the glyph in the atlas
    pub glyph_index: usize,
    /// The required offset (relative positioning) when placed
    pub offset: Vec2,
}
