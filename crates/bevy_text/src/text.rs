use bevy_asset::Handle;
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, reflect::ReflectComponent};
use bevy_hierarchy::{BuildChildren, Children, Parent};
use bevy_reflect::prelude::*;
use bevy_utils::warn_once;
use cosmic_text::{Buffer, Metrics};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{Font, TextLayoutInfo, TextRoot, TextSpanComponent};
pub use cosmic_text::{
    self, FamilyOwned as FontFamily, Stretch as FontStretch, Style as FontStyle,
    Weight as FontWeight,
};

/// Wrapper for [`cosmic_text::Buffer`]
#[derive(Deref, DerefMut, Debug, Clone)]
pub struct CosmicBuffer(pub Buffer);

impl Default for CosmicBuffer {
    fn default() -> Self {
        Self(Buffer::new_empty(Metrics::new(0.0, 0.000001)))
    }
}

/// A sub-entity of a [`TextBlock`].
///
/// Returned by [`ComputedTextBlock::entities`].
#[derive(Debug, Copy, Clone)]
pub struct TextEntity {
    /// The entity.
    pub entity: Entity,
    /// Records the hierarchy depth of the entity within a `TextBlock`.
    pub depth: usize,
}

/// Computed information for a [`TextBlock`].
///
/// Automatically updated by 2d and UI text systems.
#[derive(Component, Debug, Clone)]
pub struct ComputedTextBlock {
    /// Buffer for managing text layout and creating [`TextLayoutInfo`].
    ///
    /// This is private because buffer contents are always refreshed from ECS state when writing glyphs to
    /// `TextLayoutInfo`. If you want to control the buffer contents manually or use the `cosmic-text`
    /// editor, then you need to not use `TextBlock` and instead manually implement the conversion to
    /// `TextLayoutInfo`.
    pub(crate) buffer: CosmicBuffer,
    /// Entities for all text spans in the block, including the root-level text.
    ///
    /// The [`TextEntity::depth`] field can be used to reconstruct the hierarchy.
    pub(crate) entities: SmallVec<[TextEntity; 1]>,
    /// Flag set when any change has been made to this block that should cause it to be rerendered.
    ///
    /// Includes:
    /// - [`TextBlock`] changes.
    /// - [`TextStyle`] or `Text2d`/`Text`/`TextSpan2d`/`TextSpan` changes anywhere in the block's entity hierarchy.
    // TODO: This encompasses both structural changes like font size or justification and non-structural
    // changes like text color and font smoothing. This field currently causes UI to 'remeasure' text, even if
    // the actual changes are non-structural and can be handled by only rerendering and not remeasuring. A full
    // solution would probably require splitting TextBlock and TextStyle into structural/non-structural
    // components for more granular change detection. A cost/benefit analysis is needed.
    pub(crate) needs_rerender: bool,
}

impl ComputedTextBlock {
    /// Accesses entities in this block.
    ///
    /// Can be used to look up [`TextStyle`] components for glyphs in [`TextLayoutInfo`] using the `span_index`
    /// stored there.
    pub fn entities(&self) -> &[TextEntity] {
        &self.entities
    }

    /// Indicates if the text needs to be refreshed in [`TextLayoutInfo`].
    ///
    /// Updated automatically by [`detect_text_needs_rerender`](crate::detect_text_needs_rerender) and cleared
    /// by [`TextPipeline`] methods.
    pub fn needs_rerender(&self) -> bool {
        self.needs_rerender
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
/// A block of text is composed of text spans, which each have a separate string value and [`TextStyle`]. Text
/// spans associated with a text block are collected into [`ComputedTextBlock`] for layout, and then inserted
/// to [`TextLayoutInfo`] for rendering.
///
/// See [`Text2d`] for the core component of 2d text, and `Text` in `bevy_ui` for UI text.
#[derive(Component, Debug, Copy, Clone, Default, Reflect)]
#[reflect(Component, Default, Debug)]
#[require(ComputedTextBlock, TextLayoutInfo)]
pub struct TextBlock {
    /// The text's internal alignment.
    /// Should not affect its position within a container.
    pub justify: JustifyText,
    /// How the text should linebreak when running out of the bounds determined by `max_size`.
    pub linebreak: LineBreak,
}

impl TextBlock {
    /// Makes a new [`TextBlock`].
    pub const fn new(justify: JustifyText, linebreak: LineBreak) -> Self {
        Self { justify, linebreak }
    }

    /// Makes a new [`TextBlock`] with the specified [`JustifyText`].
    pub fn new_with_justify(justify: JustifyText) -> Self {
        Self::default().with_justify(justify)
    }

    /// Makes a new [`TextBlock`] with the specified [`LineBreak`].
    pub fn new_with_linebreak(linebreak: LineBreak) -> Self {
        Self::default().with_linebreak(linebreak)
    }

    /// Makes a new [`TextBlock`] with soft wrapping disabled.
    /// Hard wrapping, where text contains an explicit linebreak such as the escape sequence `\n`, will still occur.
    pub fn new_with_no_wrap() -> Self {
        Self::default().with_no_wrap()
    }

    /// Returns this [`TextBlock`] with the specified [`JustifyText`].
    pub const fn with_justify(mut self, justify: JustifyText) -> Self {
        self.justify = justify;
        self
    }

    /// Returns this [`TextBlock`] with the specified [`LineBreak`].
    pub const fn with_linebreak(mut self, linebreak: LineBreak) -> Self {
        self.linebreak = linebreak;
        self
    }

    /// Returns this [`TextBlock`] with soft wrapping disabled.
    /// Hard wrapping, where text contains an explicit linebreak such as the escape sequence `\n`, will still occur.
    pub const fn with_no_wrap(mut self) -> Self {
        self.linebreak = LineBreak::NoWrap;
        self
    }
}

/// Describes the horizontal alignment of multiple lines of text relative to each other.
///
/// This only affects the internal positioning of the lines of text within a text entity and
/// does not affect the text entity's position.
///
/// _Has no affect on a single line text entity._
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum JustifyText {
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

impl From<JustifyText> for cosmic_text::Align {
    fn from(justify: JustifyText) -> Self {
        match justify {
            JustifyText::Left => cosmic_text::Align::Left,
            JustifyText::Center => cosmic_text::Align::Center,
            JustifyText::Right => cosmic_text::Align::Right,
            JustifyText::Justified => cosmic_text::Align::Justified,
        }
    }
}

/// `TextStyle` determines the style of a text span within a [`TextBlock`], specifically
/// the font face, the font size, and the color.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Default, Debug)]
pub struct TextStyle {
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
    /// The color of the text for this section.
    pub color: Color,
    /// The antialiasing method to use when rendering text.
    pub font_smoothing: FontSmoothing,
}

impl TextStyle {
    /// Returns this [`TextBlock`] with the specified [`FontSmoothing`].
    pub const fn with_font_smoothing(mut self, font_smoothing: FontSmoothing) -> Self {
        self.font_smoothing = font_smoothing;
        self
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Default::default(),
            font_size: 20.0,
            color: Color::WHITE,
            font_smoothing: Default::default(),
        }
    }
}

/// Determines how lines will be broken when preventing text from running out of bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
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
#[reflect(Serialize, Deserialize)]
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

/// Provides convenience methods for spawning text blocks.
pub trait TextBuilderExt {
    /// Spawns an entire text block including the root entity as a child of the current entity.
    ///
    /// If no spans are provided then a default text entity will be spawned.
    ///
    /// Returns an [`EntityCommands`] for the root entity.
    fn spawn_text_block<R: TextRoot>(
        &mut self,
        spans: impl IntoIterator<Item = (String, TextStyle)>,
    ) -> EntityCommands;
}

impl TextBuilderExt for Commands<'_, '_> {
    fn spawn_text_block<R: TextRoot>(
        &mut self,
        spans: impl IntoIterator<Item = (String, TextStyle)>,
    ) -> EntityCommands {
        let mut spans = spans.into_iter();

        // Root of the block.
        let first = spans.next().unwrap_or_default();
        let mut ec = self.spawn((R::from(first.0), first.1));

        // Spans.
        while let Some(next) = spans.next() {
            ec.with_child((R::Span::from(next.0), next.1));
        }

        ec
    }
}

impl TextBuilderExt for EntityCommands<'_> {
    fn spawn_text_block<R: TextRoot>(
        &mut self,
        spans: impl IntoIterator<Item = (String, TextStyle)>,
    ) -> EntityCommands {
        let mut spans = spans.into_iter();

        // Root of the block.
        let first = spans.next().unwrap_or_default();
        let ec = self.with_child((R::from(first.0), first.1));

        // Spans.
        while let Some(next) = spans.next() {
            ec.with_child((R::Span::from(next.0), next.1));
        }

        ec.reborrow()
    }
}

/// Provides convenience methods for adding text spans to a text block.
pub trait TextSpanBuilderExt {
    /// Adds a flat list of spans as children of the current entity.
    fn with_spans<S: TextSpanComponent>(
        &mut self,
        spans: impl IntoIterator<Item = (String, TextStyle)>,
    ) -> &mut Self;
}

impl TextSpanBuilderExt for EntityCommands<'_> {
    fn with_spans<S: TextSpanComponent>(
        &mut self,
        spans: impl IntoIterator<Item = (String, TextStyle)>,
    ) -> &mut Self {
        for (span, style) in spans.into_iter() {
            self.with_child((S::from(span), style));
        }
        self
    }
}

/// System that detects changes to text blocks and sets `ComputedTextBlock::should_rerender`.
///
/// Generic over the root text component and text span component. For example, [`Text2d`]/[`TextSpan2d`] for 2d or
/// `Text`/`TextSpan` for UI.
pub fn detect_text_needs_rerender<Root: Component, Span: Component>(
    changed_roots: Query<
        Entity,
        (
            Or<(
                Changed<Root>,
                Changed<TextStyle>,
                Changed<TextBlock>,
                Changed<Children>,
            )>,
            With<Root>,
            With<TextStyle>,
            With<TextBlock>,
        ),
    >,
    changed_spans: Query<
        (Entity, Option<&Parent>, Has<TextBlock>),
        (
            Or<(
                Changed<Span>,
                Changed<TextStyle>,
                Changed<Children>,
                Changed<Parent>, // Included to detect broken text block hierarchies.
                Added<TextBlock>,
            )>,
            With<Span>,
            With<TextStyle>,
        ),
    >,
    mut computed: Query<(Option<&Parent>, Option<&mut ComputedTextBlock>)>,
) {
    // Root entity:
    // - Root component changed.
    // - TextStyle on root changed.
    // - TextBlock changed.
    // - Root children changed (can include additions and removals).
    for root in changed_roots.iter() {
        let Ok((_, Some(mut computed))) = computed.get_mut(root) else {
            warn_once!("found entity {:?} with a root text component ({}) but no ComputedTextBlock; this warning only \
                prints once", root, std::any::type_name::<Root>());
            continue;
        };
        computed.needs_rerender = true;
    }

    // Span entity:
    // - Span component changed.
    // - Span TextStyle changed.
    // - Span children changed (can include additions and removals).
    for (entity, maybe_span_parent, has_text_block) in changed_spans.iter() {
        if has_text_block {
            warn_once!("found entity {:?} with a text span ({}) that has a TextBlock, which should only be on root \
                text entities (that have {}); this warning only prints once",
                entity, std::any::type_name::<Span>(), std::any::type_name::<Root>());
        }

        let Some(span_parent) = maybe_span_parent else {
            warn_once!("found entity {:?} with a text span ({}) that has no parent; it should have an ancestor \
                with a root text component ({}); this warning only prints once",
                entity, std::any::type_name::<Span>(), std::any::type_name::<Root>());
            continue;
        };
        let mut parent: Entity = **span_parent;

        // Search for the nearest ancestor with ComputedTextBlock.
        // Note: We assume the perf cost from duplicate visits in the case that multiple spans in a block are visited
        // is outweighed by the expense of tracking visited spans.
        loop {
            let Ok((maybe_parent, maybe_computed)) = computed.get_mut(parent) else {
                warn_once!("found entity {:?} with a text span ({}) that is part of a broken hierarchy with a Parent \
                    component that points at non-existent entity {:?}; this warning only prints once",
                    entity, std::any::type_name::<Span>(), parent);
                break;
            };
            if let Some(mut computed) = maybe_computed {
                computed.needs_rerender = true;
                break;
            }
            let Some(next_parent) = maybe_parent else {
                warn_once!("found entity {:?} with a text span ({}) that has no ancestor with the root text \
                    component ({}); this warning only prints once",
                    entity, std::any::type_name::<Span>(), std::any::type_name::<Root>());
                break;
            };
            parent = **next_parent;
        }
    }
}
