use crate::{Font, TextLayoutInfo, TextSpanAccess, TextSpanComponent};
use bevy_asset::Handle;
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, reflect::ReflectComponent};
use bevy_reflect::prelude::*;
use bevy_utils::{default, once};
use cosmic_text::{Buffer, Metrics};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tracing::warn;

/// Wrapper for [`cosmic_text::Buffer`]
#[derive(Deref, DerefMut, Debug, Clone)]
pub struct CosmicBuffer(pub Buffer);

impl Default for CosmicBuffer {
    fn default() -> Self {
        Self(Buffer::new_empty(Metrics::new(0.0, 0.000001)))
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
/// See [`Text2d`](crate::Text2d) for the core component of 2d text, and `Text` in `bevy_ui` for UI text.
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
///
/// ```
/// # use bevy_asset::Handle;
/// # use bevy_color::Color;
/// # use bevy_color::palettes::basic::{BLUE, GREEN, RED};
/// # use bevy_ecs::{children, spawn::SpawnRelated, world::World};
/// # use bevy_text::{Font, Justify, Text2d, TextColor, TextLayout, TextFont, TextSpan};
///
/// # let font_handle: Handle<Font> = Default::default();
/// # let mut world = World::default();
/// #
/// world.spawn((
///     // `Text` or `Text2d` is needed.
///     Text2d::new("Bevy\n"),
///     // Layout of the entire block of text.
///     TextLayout::new_with_justify(Justify::Center),
///     // TextFont of this node. Won't apply to children.
///     TextFont::from_font_size(50.0),
///     // TextColor of this node. Won't apply to children.
///     TextColor(BLUE.into()),
///     // Children must be `TextSpan`, not `Text` or `Text2d`.
///     children![
///         (
///             TextSpan::new("Bevy\n"),
///             TextFont::from_font_size(40.0),
///             TextColor(RED.into()),
///         ),
///         (
///             TextSpan::new("Bevy\n"),
///             TextFont::from_font_size(30.0),
///             // Default TextColor will be inserted because TextSpan requires it.
///         ),
///         (
///             TextSpan::new("Bevy"),
///             TextColor(GREEN.into()),
///             // Default TextFont will be inserted because TextSpan requires it.
///         )
///     ],
/// ));
/// ```
#[derive(Component, Debug, Default, Clone, Deref, DerefMut, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(TextFont, TextColor)]
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

/// `TextFont` determines the style of a text span within a [`ComputedTextBlock`], specifically
/// the font face, the font size, and the color.
#[derive(Component, Clone, Debug, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, Clone)]
pub struct TextFont {
    /// The specific font face to use, as a `Handle` to a [`Font`] asset.
    ///
    /// If the `font` is not specified, then
    /// * if `default_font` feature is enabled (enabled by default in `bevy` crate),
    ///   `FiraMono-subset.ttf` compiled into the library is used.
    /// * otherwise no text will be rendered, unless a custom font is loaded into the default font
    ///   handle.
    pub font: Handle<Font>,
    /// The vertical height of rasterized glyphs in the font atlas in pixels.
    ///
    /// This is multiplied by the window scale factor and `UiScale`, but not the text entity
    /// transform or camera projection.
    ///
    /// A new font atlas is generated for every combination of font handle and scaled font size
    /// which can have a strong performance impact.
    pub font_size: f32,
    /// The vertical height of a line of text, from the top of one line to the top of the
    /// next.
    ///
    /// Defaults to `LineHeight::RelativeToFont(1.2)`
    pub line_height: LineHeight,
    /// The antialiasing method to use when rendering text.
    pub font_smoothing: FontSmoothing,
}

impl TextFont {
    /// Returns a new [`TextFont`] with the specified font face handle.
    pub fn from_font(font: Handle<Font>) -> Self {
        Self::default().with_font(font)
    }

    /// Returns a new [`TextFont`] with the specified font size.
    pub fn from_font_size(font_size: f32) -> Self {
        Self::default().with_font_size(font_size)
    }

    /// Returns a new [`TextFont`] with the specified line height.
    pub fn from_line_height(line_height: LineHeight) -> Self {
        Self::default().with_line_height(line_height)
    }

    /// Returns this [`TextFont`] with the specified font face handle.
    pub fn with_font(mut self, font: Handle<Font>) -> Self {
        self.font = font;
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

    /// Returns this [`TextFont`] with the specified [`LineHeight`].
    pub const fn with_line_height(mut self, line_height: LineHeight) -> Self {
        self.line_height = line_height;
        self
    }
}

impl From<Handle<Font>> for TextFont {
    fn from(font: Handle<Font>) -> Self {
        Self { font, ..default() }
    }
}

impl From<LineHeight> for TextFont {
    fn from(line_height: LineHeight) -> Self {
        Self {
            line_height,
            ..default()
        }
    }
}

impl Default for TextFont {
    fn default() -> Self {
        Self {
            font: Default::default(),
            font_size: 20.0,
            line_height: LineHeight::default(),
            font_smoothing: Default::default(),
        }
    }
}

/// Specifies the height of each line of text for `Text` and `Text2d`
///
/// Default is 1.2x the font size
#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
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
/// Generic over the root text component and text span component. For example, [`Text2d`](crate::Text2d)/[`TextSpan`] for
/// 2d or `Text`/[`TextSpan`] for UI.
pub fn detect_text_needs_rerender<Root: Component>(
    changed_roots: Query<
        Entity,
        (
            Or<(
                Changed<Root>,
                Changed<TextFont>,
                Changed<TextLayout>,
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
