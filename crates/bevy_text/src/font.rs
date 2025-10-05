use alloc::sync::Arc;

use bevy_asset::Asset;
use bevy_reflect::TypePath;

/// An [`Asset`] that contains the data for a loaded font, if loaded as an asset.
///
/// Loaded by [`FontLoader`](crate::FontLoader).
///
/// # A note on fonts
///
/// `Font` may differ from the everyday notion of what a "font" is.
/// A font *face* (e.g. Fira Sans Semibold Italic) is part of a font *family* (e.g. Fira Sans),
/// and is distinguished from other font faces in the same family
/// by its style (e.g. italic), its weight (e.g. bold) and its stretch (e.g. condensed).
///
/// Bevy currently loads a single font face as a single `Font` asset.
#[derive(Debug, TypePath, Clone, Asset)]
pub enum Font {
    /// Content of a font file as bytes
    Data(Arc<Vec<u8>>),
    /// References a font inserted into the font database by family, weight, stretch, and style.
    ///
    /// This can include system fonts, if enabled in [`super::TextPlugin`], or previously loaded fonts via [`Font::Data`].
    Query {
        /// A list of font families that satisfy this font requirement.
        families: Vec<Family>,
        /// Specifies the weight of glyphs in the font, their degree of blackness or stroke thickness.
        ///
        /// See [`cosmic_text::Weight`] for details.
        weight: cosmic_text::Weight,
        /// A face [width](https://docs.microsoft.com/en-us/typography/opentype/spec/os2#uswidthclass).
        ///
        /// See [`cosmic_text::Stretch`] for details.
        stretch: cosmic_text::Stretch,
        /// Allows italic or oblique faces to be selected.
        ///
        /// See [`cosmic_text::Style`] for details.
        style: cosmic_text::Style,
    },
}

/// A font family specifier, either by name or generic category.
///
/// See [`cosmic_text::Family`] for details.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// The name of a font family of choice.
    ///
    /// This must be a *Typographic Family* (ID 16) or a *Family Name* (ID 1) in terms of TrueType.
    /// Meaning you have to pass a family without any additional suffixes like _Bold_, _Italic_,
    /// _Regular_, etc.
    ///
    /// Localized names are allowed.
    Name(String),

    /// Serif fonts represent the formal text style for a script.
    Serif,

    /// Glyphs in sans-serif fonts, as the term is used in CSS, are generally low contrast
    /// and have stroke endings that are plain — without any flaring, cross stroke,
    /// or other ornamentation.
    SansSerif,

    /// Glyphs in cursive fonts generally use a more informal script style,
    /// and the result looks more like handwritten pen or brush writing than printed letterwork.
    Cursive,

    /// Fantasy fonts are primarily decorative or expressive fonts that
    /// contain decorative or expressive representations of characters.
    Fantasy,

    /// The sole criterion of a monospace font is that all glyphs have the same fixed width.
    MonoSpace,
}

impl Family {
    /// References variants to create a [`cosmic_text::Family`].
    ///
    /// This is required for querying the underlying [`cosmic_text::fontdb::Database`]
    pub fn as_fontdb_family(&self) -> cosmic_text::Family<'_> {
        match self {
            Family::Name(name) => cosmic_text::Family::Name(name),
            Family::Serif => cosmic_text::Family::Serif,
            Family::SansSerif => cosmic_text::Family::SansSerif,
            Family::Cursive => cosmic_text::Family::Cursive,
            Family::Fantasy => cosmic_text::Family::Fantasy,
            Family::MonoSpace => cosmic_text::Family::Monospace,
        }
    }
}

impl Font {
    /// Creates a [`Font`] from bytes
    pub fn try_from_bytes(
        font_data: Vec<u8>,
    ) -> Result<Self, cosmic_text::ttf_parser::FaceParsingError> {
        use cosmic_text::ttf_parser;
        ttf_parser::Face::parse(&font_data, 0)?;
        Ok(Self::Data(Arc::new(font_data)))
    }
}
