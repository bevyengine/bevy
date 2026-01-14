use crate::{Font, TextLayoutInfo, TextSpanAccess, TextSpanComponent};
use bevy_asset::Handle;
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, reflect::ReflectComponent};
use bevy_reflect::prelude::*;
use bevy_utils::{default, once};
use core::fmt::{Debug, Formatter};
use core::str::from_utf8;
use cosmic_text::{Buffer, Family, Metrics, Stretch};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use smol_str::SmolStr;
use tracing::warn;

/// Wrapper for [`cosmic_text::Buffer`]
#[derive(Deref, DerefMut, Debug, Clone)]
pub struct CosmicBuffer(pub Buffer);

impl Default for CosmicBuffer {
    fn default() -> Self {
        Self(Buffer::new_empty(Metrics::new(20.0, 20.0)))
    }
}

/// A sub-entity of a [`ComputedTextBlock`].
///
/// Returned by [`ComputedTextBlock::entities`].
#[derive(Debug, Copy, Clone, Reflect)]
#[reflect(Debug, Clone)]
pub struct TextEntity {
    /// The entity.
    pub entity: Entity,
    /// Records the hierarchy depth of the entity within a `TextLayout`.
    pub depth: usize,
}

/// Computed information for a text block.
///
/// See [`TextLayout`].
///
/// Automatically updated by 2d and UI text systems.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ComputedTextBlock {
    /// Buffer for managing text layout and creating [`TextLayoutInfo`].
    ///
    /// This is private because buffer contents are always refreshed from ECS state when writing glyphs to
    /// `TextLayoutInfo`. If you want to control the buffer contents manually or use the `cosmic-text`
    /// editor, then you need to not use `TextLayout` and instead manually implement the conversion to
    /// `TextLayoutInfo`.
    #[reflect(ignore, clone)]
    pub(crate) buffer: CosmicBuffer,
    /// Entities for all text spans in the block, including the root-level text.
    ///
    /// The [`TextEntity::depth`] field can be used to reconstruct the hierarchy.
    pub(crate) entities: SmallVec<[TextEntity; 1]>,
    /// Flag set when any change has been made to this block that should cause it to be rerendered.
    ///
    /// Includes:
    /// - [`TextLayout`] changes.
    /// - [`TextFont`] or `Text2d`/`Text`/`TextSpan` changes anywhere in the block's entity hierarchy.
    // TODO: This encompasses both structural changes like font size or justification and non-structural
    // changes like text color and font smoothing. This field currently causes UI to 'remeasure' text, even if
    // the actual changes are non-structural and can be handled by only rerendering and not remeasuring. A full
    // solution would probably require splitting TextLayout and TextFont into structural/non-structural
    // components for more granular change detection. A cost/benefit analysis is needed.
    pub(crate) needs_rerender: bool,
}

impl ComputedTextBlock {
    /// Accesses entities in this block.
    ///
    /// Can be used to look up [`TextFont`] components for glyphs in [`TextLayoutInfo`] using the `span_index`
    /// stored there.
    pub fn entities(&self) -> &[TextEntity] {
        &self.entities
    }

    /// Indicates if the text needs to be refreshed in [`TextLayoutInfo`].
    ///
    /// Updated automatically by [`detect_text_needs_rerender`] and cleared
    /// by [`TextPipeline`](crate::TextPipeline) methods.
    pub fn needs_rerender(&self) -> bool {
        self.needs_rerender
    }
    /// Accesses the underlying buffer which can be used for `cosmic-text` APIs such as accessing layout information
    /// or calculating a cursor position.
    ///
    /// Mutable access is not offered because changes would be overwritten during the automated layout calculation.
    /// If you want to control the buffer contents manually or use the `cosmic-text`
    /// editor, then you need to not use `TextLayout` and instead manually implement the conversion to
    /// `TextLayoutInfo`.
    pub fn buffer(&self) -> &CosmicBuffer {
        &self.buffer
    }
}

impl Default for ComputedTextBlock {
    fn default() -> Self {
        Self {
            buffer: CosmicBuffer::default(),
            entities: SmallVec::default(),
            needs_rerender: true,
        }
    }
}

/// Component with text format settings for a block of text.
///
/// A block of text is composed of text spans, which each have a separate string value and [`TextFont`]. Text
/// spans associated with a text block are collected into [`ComputedTextBlock`] for layout, and then inserted
/// to [`TextLayoutInfo`] for rendering.
///
/// See `Text2d` in `bevy_sprite` for the core component of 2d text, and `Text` in `bevy_ui` for UI text.
#[derive(Component, Debug, Copy, Clone, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(ComputedTextBlock, TextLayoutInfo)]
pub struct TextLayout {
    /// The text's internal alignment.
    /// Should not affect its position within a container.
    pub justify: Justify,
    /// How the text should linebreak when running out of the bounds determined by `max_size`.
    pub linebreak: LineBreak,
}

impl TextLayout {
    /// Makes a new [`TextLayout`].
    pub const fn new(justify: Justify, linebreak: LineBreak) -> Self {
        Self { justify, linebreak }
    }

    /// Makes a new [`TextLayout`] with the specified [`Justify`].
    pub fn new_with_justify(justify: Justify) -> Self {
        Self::default().with_justify(justify)
    }

    /// Makes a new [`TextLayout`] with the specified [`LineBreak`].
    pub fn new_with_linebreak(linebreak: LineBreak) -> Self {
        Self::default().with_linebreak(linebreak)
    }

    /// Makes a new [`TextLayout`] with soft wrapping disabled.
    /// Hard wrapping, where text contains an explicit linebreak such as the escape sequence `\n`, will still occur.
    pub fn new_with_no_wrap() -> Self {
        Self::default().with_no_wrap()
    }

    /// Returns this [`TextLayout`] with the specified [`Justify`].
    pub const fn with_justify(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    /// Returns this [`TextLayout`] with the specified [`LineBreak`].
    pub const fn with_linebreak(mut self, linebreak: LineBreak) -> Self {
        self.linebreak = linebreak;
        self
    }

    /// Returns this [`TextLayout`] with soft wrapping disabled.
    /// Hard wrapping, where text contains an explicit linebreak such as the escape sequence `\n`, will still occur.
    pub const fn with_no_wrap(mut self) -> Self {
        self.linebreak = LineBreak::NoWrap;
        self
    }
}

/// A span of text in a tree of spans.
///
/// A `TextSpan` is only valid when it exists as a child of a parent that has either `Text` or
/// `Text2d`. The parent's `Text` / `Text2d` component contains the base text content. Any children
/// with `TextSpan` extend this text by appending their content to the parent's text in sequence to
/// form a [`ComputedTextBlock`]. The parent's [`TextLayout`] determines the layout of the block
/// but each node has its own [`TextFont`] and [`TextColor`].
#[derive(Component, Debug, Default, Clone, Deref, DerefMut, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(TextFont, TextColor, LineHeight)]
pub struct TextSpan(pub String);

impl TextSpan {
    /// Makes a new text span component.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl TextSpanComponent for TextSpan {}

impl TextSpanAccess for TextSpan {
    fn read_span(&self) -> &str {
        self.as_str()
    }
    fn write_span(&mut self) -> &mut String {
        &mut *self
    }
}

impl From<&str> for TextSpan {
    fn from(value: &str) -> Self {
        Self(String::from(value))
    }
}

impl From<String> for TextSpan {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Describes the horizontal alignment of multiple lines of text relative to each other.
///
/// This only affects the internal positioning of the lines of text within a text entity and
/// does not affect the text entity's position.
///
/// _Has no affect on a single line text entity_, unless used together with a
/// [`TextBounds`](super::bounds::TextBounds) component with an explicit `width` value.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize, Clone, PartialEq, Hash)]
#[doc(alias = "JustifyText")]
pub enum Justify {
    /// Leftmost character is immediately to the right of the render position.
    /// Bounds start from the render position and advance rightwards.
    #[default]
    Left,
    /// Leftmost & rightmost characters are equidistant to the render position.
    /// Bounds start from the render position and advance equally left & right.
    Center,
    /// Rightmost character is immediately to the left of the render position.
    /// Bounds start from the render position and advance leftwards.
    Right,
    /// Words are spaced so that leftmost & rightmost characters
    /// align with their margins.
    /// Bounds start from the render position and advance equally left & right.
    Justified,
}

impl From<Justify> for cosmic_text::Align {
    fn from(justify: Justify) -> Self {
        match justify {
            Justify::Left => cosmic_text::Align::Left,
            Justify::Center => cosmic_text::Align::Center,
            Justify::Right => cosmic_text::Align::Right,
            Justify::Justified => cosmic_text::Align::Justified,
        }
    }
}

#[derive(Clone, Debug, Reflect, PartialEq)]
/// Determines how the font face for a text sections is selected.
///
/// A `FontSource` can be a handle to a font asset, a font family name,
/// or a generic font category that is resolved using Cosmic Text's font database.
///
/// The `CosmicFontSystem` resource can be used to change the font family
/// associated to a generic font variant:
/// ```
/// # use bevy_ecs::world::World;
/// # use bevy_text::CosmicFontSystem;
/// # use bevy_ui::prelude::Text;
/// # let mut world = World::default();
/// let mut font_system = world.resource_mut::<CosmicFontSystem>();
///
/// font_system.db_mut().set_serif_family("Allegro");
/// font_system.db_mut().set_sans_serif_family("Encode Sans");
/// font_system.db_mut().set_cursive_family("Cedarville Cursive");
/// font_system.db_mut().set_fantasy_family("Argusho");
/// font_system.db_mut().set_monospace_family("Lucida Console");
///
/// // `CosmicFontSystem::get_family` can be used to look up the name
/// // of a `FontSource`'s associated family
/// let family_name = font_system.get_family(FontSource::Serif);
/// assert_eq!(family_name.as_str(), "Allegro");
/// ```
pub enum FontSource {
    /// Use a specific font face referenced by a [`Font`] asset handle.
    ///
    /// If the default font handle is used, then
    /// * if `default_font` feature is enabled (enabled by default in `bevy` crate),
    ///   `FiraMono-subset.ttf` compiled into the library is used.
    /// * otherwise no text will be rendered, unless a custom font is loaded into the default font
    ///   handle.
    Handle(Handle<Font>),
    /// Resolve the font by family name using the font database.
    Family(SmolStr),
    /// Fonts with serifs — small decorative strokes at the ends of letterforms.
    ///
    /// Serif fonts are typically used for long passages of text and represent
    /// a more traditional or formal typographic style.
    Serif,
    /// Fonts without serifs.
    ///
    /// Sans-serif fonts generally have low stroke contrast and plain stroke
    /// endings, making them common for UI text and on-screen reading.
    SansSerif,
    /// Fonts that use a cursive or handwritten style.
    ///
    /// Glyphs often resemble connected or flowing pen or brush strokes rather
    /// than printed letterforms.
    Cursive,
    /// Decorative or expressive fonts.
    ///
    /// Fantasy fonts are primarily intended for display purposes and may
    /// prioritize visual style over readability.
    Fantasy,
    /// Fonts in which all glyphs have the same fixed advance width.
    ///
    /// Monospace fonts are commonly used for code, tabular data, and text
    /// where vertical alignment is important.
    Monospace,
}

impl FontSource {
    /// Returns this `FontSource` as a `fontdb` family, or `None`
    /// if this source is a `Handle`.
    pub(crate) fn as_family<'a>(&'a self) -> Option<Family<'a>> {
        Some(match self {
            FontSource::Family(family) => Family::Name(family.as_str()),
            FontSource::Serif => Family::Serif,
            FontSource::SansSerif => Family::SansSerif,
            FontSource::Cursive => Family::Cursive,
            FontSource::Fantasy => Family::Fantasy,
            FontSource::Monospace => Family::Monospace,
            _ => return None,
        })
    }
}

impl Default for FontSource {
    fn default() -> Self {
        Self::Handle(Handle::default())
    }
}

impl From<Handle<Font>> for FontSource {
    fn from(handle: Handle<Font>) -> Self {
        Self::Handle(handle)
    }
}

impl From<&Handle<Font>> for FontSource {
    fn from(handle: &Handle<Font>) -> Self {
        Self::Handle(handle.clone())
    }
}

impl From<SmolStr> for FontSource {
    fn from(family: SmolStr) -> Self {
        FontSource::Family(family)
    }
}

impl From<&str> for FontSource {
    fn from(family: &str) -> Self {
        FontSource::Family(family.into())
    }
}

/// `TextFont` determines the style of a text span within a [`ComputedTextBlock`], specifically
/// the font face, the font size, the line height, and the antialiasing method.
#[derive(Component, Clone, Debug, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, Clone)]
pub struct TextFont {
    /// Specifies the font face used for this text section.
    ///
    /// A `FontSource` can be a handle to a font asset, a font family name,
    /// or a generic font category that is resolved using Cosmic Text's font database.
    pub font: FontSource,
    /// The vertical height of rasterized glyphs in the font atlas in pixels.
    ///
    /// This is multiplied by the window scale factor and `UiScale`, but not the text entity
    /// transform or camera projection.
    ///
    /// A new font atlas is generated for every combination of font handle and scaled font size
    /// which can have a strong performance impact.
    pub font_size: f32,
    /// How thick or bold the strokes of a font appear.
    ///
    /// Font weights can be any value between 1 and 1000, inclusive.
    ///
    /// Only supports variable weight fonts.
    pub weight: FontWeight,
    /// How condensed or expanded the glyphs appear horizontally.
    pub width: FontWidth,
    /// The slant style of a font face: normal, italic, or oblique.
    pub style: FontStyle,
    /// The antialiasing method to use when rendering text.
    pub font_smoothing: FontSmoothing,
    /// OpenType features for .otf fonts that support them.
    pub font_features: FontFeatures,
}

impl TextFont {
    /// Returns a new [`TextFont`] with the specified font size.
    pub fn from_font_size(font_size: f32) -> Self {
        Self::default().with_font_size(font_size)
    }

    /// Returns this [`TextFont`] with the specified font face handle.
    pub fn with_font(mut self, font: Handle<Font>) -> Self {
        self.font = FontSource::Handle(font);
        self
    }

    /// Returns this [`TextFont`] with the specified font family.
    pub fn with_family(mut self, family: impl Into<SmolStr>) -> Self {
        self.font = FontSource::Family(family.into());
        self
    }

    /// Returns this [`TextFont`] with the specified font size.
    pub const fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    /// Returns this [`TextFont`] with the specified [`FontSmoothing`].
    pub const fn with_font_smoothing(mut self, font_smoothing: FontSmoothing) -> Self {
        self.font_smoothing = font_smoothing;
        self
    }
}

impl<T: Into<FontSource>> From<T> for TextFont {
    fn from(source: T) -> Self {
        Self {
            font: source.into(),
            ..default()
        }
    }
}

impl Default for TextFont {
    fn default() -> Self {
        Self {
            font: Default::default(),
            font_size: 20.0,
            style: FontStyle::Normal,
            weight: FontWeight::NORMAL,
            width: FontWidth::NORMAL,
            font_features: FontFeatures::default(),
            font_smoothing: Default::default(),
        }
    }
}

/// How thick or bold the strokes of a font appear.
///
/// Valid font weights range from 1 to 1000, inclusive.
/// Weights above 1000 are clamped to 1000.
/// A weight of 0 is treated as [`FontWeight::DEFAULT`].
///
/// `<https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/font-weight>`
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub struct FontWeight(pub u16);

impl FontWeight {
    /// Weight 100.
    pub const THIN: FontWeight = FontWeight(100);

    /// Weight 200.
    pub const EXTRA_LIGHT: FontWeight = FontWeight(200);

    /// Weight 300.
    pub const LIGHT: FontWeight = FontWeight(300);

    /// Weight 400.
    pub const NORMAL: FontWeight = FontWeight(400);

    /// Weight 500.
    pub const MEDIUM: FontWeight = FontWeight(500);

    /// Weight 600.
    pub const SEMIBOLD: FontWeight = FontWeight(600);

    /// Weight 700.
    pub const BOLD: FontWeight = FontWeight(700);

    /// Weight 800
    pub const EXTRA_BOLD: FontWeight = FontWeight(800);

    /// Weight 900.
    pub const BLACK: FontWeight = FontWeight(900);

    /// Weight 950.
    pub const EXTRA_BLACK: FontWeight = FontWeight(950);

    /// The default font weight.
    pub const DEFAULT: FontWeight = Self::NORMAL;

    /// Clamp the weight value to between 1 and 1000.
    /// Values of 0 are mapped to `Weight::DEFAULT`.
    pub const fn clamp(mut self) -> Self {
        if self.0 == 0 {
            self = Self::DEFAULT;
        } else if 1000 < self.0 {
            self.0 = 1000;
        }
        Self(self.0)
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<FontWeight> for cosmic_text::Weight {
    fn from(value: FontWeight) -> Self {
        cosmic_text::Weight(value.clamp().0)
    }
}

/// `<https://docs.microsoft.com/en-us/typography/opentype/spec/os2#uswidthclass>`
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, Reflect)]
pub struct FontWidth(u16);

impl FontWidth {
    /// 50% of normal width.
    pub const ULTRA_CONDENSED: Self = Self(1);

    /// 62.5% of normal width.
    pub const EXTRA_CONDENSED: Self = Self(2);

    /// 75% of normal width.
    pub const CONDENSED: Self = Self(3);

    /// 87.5% of normal width.
    pub const SEMI_CONDENSED: Self = Self(4);

    /// 100% of normal width. This is the default.
    pub const NORMAL: Self = Self(5);

    /// 112.5% of normal width.
    pub const SEMI_EXPANDED: Self = Self(6);

    /// 125% of normal width.
    pub const EXPANDED: Self = Self(7);

    /// 150% of normal width.
    pub const EXTRA_EXPANDED: Self = Self(8);

    /// 200% of normal width.
    pub const ULTRA_EXPANDED: Self = Self(9);
}

impl Default for FontWidth {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl From<FontWidth> for Stretch {
    fn from(value: FontWidth) -> Self {
        match value.0 {
            1 => Stretch::UltraCondensed,
            2 => Stretch::ExtraCondensed,
            3 => Stretch::Condensed,
            4 => Stretch::SemiCondensed,
            6 => Stretch::SemiExpanded,
            7 => Stretch::Expanded,
            8 => Stretch::ExtraExpanded,
            9 => Stretch::UltraExpanded,
            _ => Stretch::Normal,
        }
    }
}

/// The slant style of a font face: normal, italic, or oblique.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Hash, Reflect)]
pub enum FontStyle {
    /// A face that is neither italic nor obliqued.
    #[default]
    Normal,
    /// A form that is generally cursive in nature.
    Italic,
    /// A typically sloped version of the regular face.
    Oblique,
}

impl From<FontStyle> for cosmic_text::Style {
    fn from(value: FontStyle) -> Self {
        match value {
            FontStyle::Normal => cosmic_text::Style::Normal,
            FontStyle::Italic => cosmic_text::Style::Italic,
            FontStyle::Oblique => cosmic_text::Style::Oblique,
        }
    }
}

/// An OpenType font feature tag.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub struct FontFeatureTag([u8; 4]);

impl FontFeatureTag {
    /// Replaces character combinations like fi, fl with ligatures.
    pub const STANDARD_LIGATURES: FontFeatureTag = FontFeatureTag::new(b"liga");

    /// Enables ligatures based on character context.
    pub const CONTEXTUAL_LIGATURES: FontFeatureTag = FontFeatureTag::new(b"clig");

    /// Enables optional ligatures for stylistic use (e.g., ct, st).
    pub const DISCRETIONARY_LIGATURES: FontFeatureTag = FontFeatureTag::new(b"dlig");

    /// Adjust glyph shapes based on surrounding letters.
    pub const CONTEXTUAL_ALTERNATES: FontFeatureTag = FontFeatureTag::new(b"calt");

    /// Use alternate glyph designs.
    pub const STYLISTIC_ALTERNATES: FontFeatureTag = FontFeatureTag::new(b"salt");

    /// Replaces lowercase letters with small caps.
    pub const SMALL_CAPS: FontFeatureTag = FontFeatureTag::new(b"smcp");

    /// Replaces uppercase letters with small caps.
    pub const CAPS_TO_SMALL_CAPS: FontFeatureTag = FontFeatureTag::new(b"c2sc");

    /// Replaces characters with swash versions (often decorative).
    pub const SWASH: FontFeatureTag = FontFeatureTag::new(b"swsh");

    /// Enables alternate glyphs for large sizes or titles.
    pub const TITLING_ALTERNATES: FontFeatureTag = FontFeatureTag::new(b"titl");

    /// Converts numbers like 1/2 into true fractions (½).
    pub const FRACTIONS: FontFeatureTag = FontFeatureTag::new(b"frac");

    /// Formats characters like 1st, 2nd properly.
    pub const ORDINALS: FontFeatureTag = FontFeatureTag::new(b"ordn");

    /// Uses a slashed version of zero (0) to differentiate from O.
    pub const SLASHED_ZERO: FontFeatureTag = FontFeatureTag::new(b"ordn");

    /// Replaces figures with superscript figures, e.g. for indicating footnotes.
    pub const SUPERSCRIPT: FontFeatureTag = FontFeatureTag::new(b"sups");

    /// Replaces figures with subscript figures.
    pub const SUBSCRIPT: FontFeatureTag = FontFeatureTag::new(b"subs");

    /// Changes numbers to "oldstyle" form, which fit better in the flow of sentences or other text.
    pub const OLDSTYLE_FIGURES: FontFeatureTag = FontFeatureTag::new(b"onum");

    /// Changes numbers to "lining" form, which are better suited for standalone numbers. When
    /// enabled, the bottom of all numbers will be aligned with each other.
    pub const LINING_FIGURES: FontFeatureTag = FontFeatureTag::new(b"lnum");

    /// Changes numbers to be of proportional width. When enabled, numbers may have varying widths.
    pub const PROPORTIONAL_FIGURES: FontFeatureTag = FontFeatureTag::new(b"pnum");

    /// Changes numbers to be of uniform (tabular) width. When enabled, all numbers will have the
    /// same width.
    pub const TABULAR_FIGURES: FontFeatureTag = FontFeatureTag::new(b"tnum");

    /// Varies the stroke thickness. Valid values are in the range of 1 to 1000, inclusive.
    pub const WEIGHT: FontFeatureTag = FontFeatureTag::new(b"wght");

    /// Varies the width of text from narrower to wider. Must be a value greater than 0. A value of
    /// 100 is typically considered standard width.
    pub const WIDTH: FontFeatureTag = FontFeatureTag::new(b"wdth");

    /// Varies between upright and slanted text. Must be a value greater than -90 and less than +90.
    /// A value of 0 is upright.
    pub const SLANT: FontFeatureTag = FontFeatureTag::new(b"slnt");

    /// Create a new [`FontFeatureTag`] from raw bytes.
    pub const fn new(src: &[u8; 4]) -> Self {
        Self(*src)
    }
}

impl Debug for FontFeatureTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        // OpenType tags are always ASCII, so this match will succeed for valid tags. This gives us
        // human-readable debug output, e.g. FontFeatureTag("liga").
        match from_utf8(&self.0) {
            Ok(s) => write!(f, "FontFeatureTag(\"{}\")", s),
            Err(_) => write!(f, "FontFeatureTag({:?})", self.0),
        }
    }
}

/// OpenType features for .otf fonts that support them.
///
/// Examples features include ligatures, small-caps, and fractional number display. For the complete
/// list of OpenType features, see the spec at
/// `<https://learn.microsoft.com/en-us/typography/opentype/spec/featurelist>`.
///
/// # Usage:
/// ```
/// use bevy_text::{FontFeatureTag, FontFeatures};
///
/// // Create using the builder
/// let font_features = FontFeatures::builder()
///   .enable(FontFeatureTag::STANDARD_LIGATURES)
///   .set(FontFeatureTag::WEIGHT, 300)
///   .build();
///
/// // Create from a list
/// let more_font_features: FontFeatures = [
///   FontFeatureTag::STANDARD_LIGATURES,
///   FontFeatureTag::OLDSTYLE_FIGURES,
///   FontFeatureTag::TABULAR_FIGURES
/// ].into();
/// ```
#[derive(Clone, Debug, Default, Reflect, PartialEq)]
pub struct FontFeatures {
    features: Vec<(FontFeatureTag, u32)>,
}

impl FontFeatures {
    /// Create a new [`FontFeaturesBuilder`].
    pub fn builder() -> FontFeaturesBuilder {
        FontFeaturesBuilder::default()
    }
}

/// A builder for [`FontFeatures`].
#[derive(Clone, Default)]
pub struct FontFeaturesBuilder {
    features: Vec<(FontFeatureTag, u32)>,
}

impl FontFeaturesBuilder {
    /// Enable an OpenType feature.
    ///
    /// Most OpenType features are on/off switches, so this is a convenience method that sets the
    /// feature's value to "1" (enabled). For non-boolean features, see [`FontFeaturesBuilder::set`].
    pub fn enable(self, feature_tag: FontFeatureTag) -> Self {
        self.set(feature_tag, 1)
    }

    /// Set an OpenType feature to a specific value.
    ///
    /// For most features, the [`FontFeaturesBuilder::enable`] method should be used instead. A few
    /// features, such as "wght", take numeric values, so this method may be used for these cases.
    pub fn set(mut self, feature_tag: FontFeatureTag, value: u32) -> Self {
        self.features.push((feature_tag, value));
        self
    }

    /// Build a [`FontFeatures`] from the values set within this builder.
    pub fn build(self) -> FontFeatures {
        FontFeatures {
            features: self.features,
        }
    }
}

/// Allow [`FontFeatures`] to be built from a list. This is suitable for the standard case when each
/// listed feature is a boolean type. If any features require a numeric value (like "wght"), use
/// [`FontFeaturesBuilder`] instead.
impl<T> From<T> for FontFeatures
where
    T: IntoIterator<Item = FontFeatureTag>,
{
    fn from(value: T) -> Self {
        FontFeatures {
            features: value.into_iter().map(|x| (x, 1)).collect(),
        }
    }
}

impl From<&FontFeatures> for cosmic_text::FontFeatures {
    fn from(font_features: &FontFeatures) -> Self {
        cosmic_text::FontFeatures {
            features: font_features
                .features
                .iter()
                .map(|(tag, value)| cosmic_text::Feature {
                    tag: cosmic_text::FeatureTag::new(&tag.0),
                    value: *value,
                })
                .collect(),
        }
    }
}

/// Specifies the height of each line of text for `Text` and `Text2d`
///
/// Default is 1.2x the font size
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Debug, Clone, PartialEq)]
pub enum LineHeight {
    /// Set line height to a specific number of pixels
    Px(f32),
    /// Set line height to a multiple of the font size
    RelativeToFont(f32),
}

impl LineHeight {
    pub(crate) fn eval(self, font_size: f32) -> f32 {
        match self {
            LineHeight::Px(px) => px,
            LineHeight::RelativeToFont(scale) => scale * font_size,
        }
    }
}

impl Default for LineHeight {
    fn default() -> Self {
        LineHeight::RelativeToFont(1.2)
    }
}

/// The color of the text for this section.
#[derive(Component, Copy, Clone, Debug, Deref, DerefMut, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct TextColor(pub Color);

impl Default for TextColor {
    fn default() -> Self {
        Self::WHITE
    }
}

impl<T: Into<Color>> From<T> for TextColor {
    fn from(color: T) -> Self {
        Self(color.into())
    }
}

impl TextColor {
    /// Black colored text
    pub const BLACK: Self = TextColor(Color::BLACK);
    /// White colored text
    pub const WHITE: Self = TextColor(Color::WHITE);
}

/// The background color of the text for this section.
#[derive(Component, Copy, Clone, Debug, Deref, DerefMut, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct TextBackgroundColor(pub Color);

impl Default for TextBackgroundColor {
    fn default() -> Self {
        Self(Color::BLACK)
    }
}

impl<T: Into<Color>> From<T> for TextBackgroundColor {
    fn from(color: T) -> Self {
        Self(color.into())
    }
}

impl TextBackgroundColor {
    /// Black background
    pub const BLACK: Self = TextBackgroundColor(Color::BLACK);
    /// White background
    pub const WHITE: Self = TextBackgroundColor(Color::WHITE);
}

/// Determines how lines will be broken when preventing text from running out of bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize, Clone, PartialEq, Hash, Default)]
pub enum LineBreak {
    /// Uses the [Unicode Line Breaking Algorithm](https://www.unicode.org/reports/tr14/).
    /// Lines will be broken up at the nearest suitable word boundary, usually a space.
    /// This behavior suits most cases, as it keeps words intact across linebreaks.
    #[default]
    WordBoundary,
    /// Lines will be broken without discrimination on any character that would leave bounds.
    /// This is closer to the behavior one might expect from text in a terminal.
    /// However it may lead to words being broken up across linebreaks.
    AnyCharacter,
    /// Wraps at the word level, or fallback to character level if a word can’t fit on a line by itself
    WordOrCharacter,
    /// No soft wrapping, where text is automatically broken up into separate lines when it overflows a boundary, will ever occur.
    /// Hard wrapping, where text contains an explicit linebreak such as the escape sequence `\n`, is still enabled.
    NoWrap,
}

/// A text entity with this component is drawn with strikethrough.
#[derive(Component, Copy, Clone, Debug, Reflect, Default, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize, Clone, Default)]
pub struct Strikethrough;

/// Color for the text's strikethrough. If this component is not present, its `TextColor` will be used.
#[derive(Component, Copy, Clone, Debug, Deref, DerefMut, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct StrikethroughColor(pub Color);

impl Default for StrikethroughColor {
    fn default() -> Self {
        Self(Color::WHITE)
    }
}

impl<T: Into<Color>> From<T> for StrikethroughColor {
    fn from(color: T) -> Self {
        Self(color.into())
    }
}

/// Add to a text entity to draw its text with underline.
#[derive(Component, Copy, Clone, Debug, Reflect, Default, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize, Clone, Default)]
pub struct Underline;

/// Color for the text's underline. If this component is not present, its `TextColor` will be used.
#[derive(Component, Copy, Clone, Debug, Deref, DerefMut, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct UnderlineColor(pub Color);

impl Default for UnderlineColor {
    fn default() -> Self {
        Self(Color::WHITE)
    }
}

impl<T: Into<Color>> From<T> for UnderlineColor {
    fn from(color: T) -> Self {
        Self(color.into())
    }
}

/// Determines which antialiasing method to use when rendering text. By default, text is
/// rendered with grayscale antialiasing, but this can be changed to achieve a pixelated look.
///
/// **Note:** Subpixel antialiasing is not currently supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize, Clone, PartialEq, Hash, Default)]
#[doc(alias = "antialiasing")]
#[doc(alias = "pixelated")]
pub enum FontSmoothing {
    /// No antialiasing. Useful for when you want to render text with a pixel art aesthetic.
    ///
    /// Combine this with `UiAntiAlias::Off` and `Msaa::Off` on your 2D camera for a fully pixelated look.
    ///
    /// **Note:** Due to limitations of the underlying text rendering library,
    /// this may require specially-crafted pixel fonts to look good, especially at small sizes.
    None,
    /// The default grayscale antialiasing. Produces text that looks smooth,
    /// even at small font sizes and low resolutions with modern vector fonts.
    #[default]
    AntiAliased,
    // TODO: Add subpixel antialias support
    // SubpixelAntiAliased,
}

/// System that detects changes to text blocks and sets `ComputedTextBlock::should_rerender`.
///
/// Generic over the root text component and text span component. For example, `Text2d`/[`TextSpan`] for
/// 2d or `Text`/[`TextSpan`] for UI.
pub fn detect_text_needs_rerender<Root: Component>(
    changed_roots: Query<
        Entity,
        (
            Or<(
                Changed<Root>,
                Changed<TextFont>,
                Changed<TextLayout>,
                Changed<LineHeight>,
                Changed<Children>,
            )>,
            With<Root>,
            With<TextFont>,
            With<TextLayout>,
        ),
    >,
    changed_spans: Query<
        (Entity, Option<&ChildOf>, Has<TextLayout>),
        (
            Or<(
                Changed<TextSpan>,
                Changed<TextFont>,
                Changed<LineHeight>,
                Changed<Children>,
                Changed<ChildOf>, // Included to detect broken text block hierarchies.
                Added<TextLayout>,
            )>,
            With<TextSpan>,
            With<TextFont>,
        ),
    >,
    mut computed: Query<(
        Option<&ChildOf>,
        Option<&mut ComputedTextBlock>,
        Has<TextSpan>,
    )>,
) {
    // Root entity:
    // - Root component changed.
    // - TextFont on root changed.
    // - TextLayout changed.
    // - Root children changed (can include additions and removals).
    for root in changed_roots.iter() {
        let Ok((_, Some(mut computed), _)) = computed.get_mut(root) else {
            once!(warn!("found entity {} with a root text component ({}) but no ComputedTextBlock; this warning only \
                prints once", root, core::any::type_name::<Root>()));
            continue;
        };
        computed.needs_rerender = true;
    }

    // Span entity:
    // - Span component changed.
    // - Span TextFont changed.
    // - Span children changed (can include additions and removals).
    for (entity, maybe_span_child_of, has_text_block) in changed_spans.iter() {
        if has_text_block {
            once!(warn!("found entity {} with a TextSpan that has a TextLayout, which should only be on root \
                text entities (that have {}); this warning only prints once",
                entity, core::any::type_name::<Root>()));
        }

        let Some(span_child_of) = maybe_span_child_of else {
            once!(warn!(
                "found entity {} with a TextSpan that has no parent; it should have an ancestor \
                with a root text component ({}); this warning only prints once",
                entity,
                core::any::type_name::<Root>()
            ));
            continue;
        };
        let mut parent: Entity = span_child_of.parent();

        // Search for the nearest ancestor with ComputedTextBlock.
        // Note: We assume the perf cost from duplicate visits in the case that multiple spans in a block are visited
        // is outweighed by the expense of tracking visited spans.
        loop {
            let Ok((maybe_child_of, maybe_computed, has_span)) = computed.get_mut(parent) else {
                once!(warn!("found entity {} with a TextSpan that is part of a broken hierarchy with a ChildOf \
                    component that points at non-existent entity {}; this warning only prints once",
                    entity, parent));
                break;
            };
            if let Some(mut computed) = maybe_computed {
                computed.needs_rerender = true;
                break;
            }
            if !has_span {
                once!(warn!("found entity {} with a TextSpan that has an ancestor ({}) that does not have a text \
                span component or a ComputedTextBlock component; this warning only prints once",
                    entity, parent));
                break;
            }
            let Some(next_child_of) = maybe_child_of else {
                once!(warn!(
                    "found entity {} with a TextSpan that has no ancestor with the root text \
                    component ({}); this warning only prints once",
                    entity,
                    core::any::type_name::<Root>()
                ));
                break;
            };
            parent = next_child_of.parent();
        }
    }
}

#[derive(Component, Debug, Copy, Clone, Default, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, Clone, PartialEq)]
/// Font hinting strategy.
///
/// The text bounds can underflow or overflow slightly with `FontHinting::Enabled`.
///
/// <https://docs.rs/cosmic-text/latest/cosmic_text/enum.Hinting.html>
pub enum FontHinting {
    #[default]
    /// Glyphs will have subpixel coordinates.
    Disabled,
    /// Glyphs will be snapped to integral coordinates in the X-axis during layout.
    ///
    /// The text bounds can underflow or overflow slightly with this enabled.
    Enabled,
}

impl From<FontHinting> for cosmic_text::Hinting {
    fn from(value: FontHinting) -> Self {
        match value {
            FontHinting::Disabled => cosmic_text::Hinting::Disabled,
            FontHinting::Enabled => cosmic_text::Hinting::Enabled,
        }
    }
}
