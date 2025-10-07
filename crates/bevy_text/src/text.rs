use crate::{
    style::ComputedTextStyle, Font, InheritableTextStyle, InheritedTextStyle, TextLayoutInfo,
};
use bevy_asset::{AssetId, Handle};
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, reflect::ReflectComponent, relationship::Relationship};
use bevy_math::Vec2;
use bevy_reflect::prelude::*;
use cosmic_text::{Buffer, Metrics};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

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
    /// - [`FontFace`] or `Text2d`/`Text` changes anywhere in the block's entity hierarchy.
    // TODO: This encompasses both structural changes like font size or justification and non-structural
    // changes like text color and font smoothing. This field currently causes UI to 'remeasure' text, even if
    // the actual changes are non-structural and can be handled by only rerendering and not remeasuring. A full
    // solution would probably require splitting TextLayout and FontFace into structural/non-structural
    // components for more granular change detection. A cost/benefit analysis is needed.
    pub(crate) needs_rerender: bool,
}

impl ComputedTextBlock {
    /// Accesses entities in this block.
    ///
    /// Can be used to look up [`FontFace`] components for glyphs in [`TextLayoutInfo`] using the `span_index`
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
/// A block of text is composed of text spans, which each have a separate string value and [`FontFace`]. Text
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

/// The specific font face to use, as a `Handle` to a [`Font`] asset.
///
/// If the `font` is not specified, then
/// * if `default_font` feature is enabled (enabled by default in `bevy` crate),
///   `FiraMono-subset.ttf` compiled into the library is used.
/// * otherwise no text will be rendered, unless a custom font is loaded into the default font
///   handle.
#[derive(Default, Component, Clone, Debug, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, Clone)]
pub struct FontFace(pub Handle<Font>);

impl From<Handle<Font>> for FontFace {
    fn from(font: Handle<Font>) -> Self {
        Self(font)
    }
}

impl InheritableTextStyle for FontFace {
    type Inherited = AssetId<Font>;

    fn to_inherited(&self) -> InheritedTextStyle<Self::Inherited> {
        InheritedTextStyle(self.0.id())
    }
}

/// The vertical height of rasterized glyphs in the font atlas in pixels.
///
/// This is multiplied by the window scale factor and `UiScale`, but not the text entity
/// transform or camera projection.
///
/// A new font atlas is generated for every combination of font handle and scaled font size
/// which can have a strong performance impact.
#[derive(Component, Copy, Clone, Debug)]
pub enum FontSize {
    /// Font Size in logical pixels.
    Px(f32),
    /// Font Size relative to the size of the default font.
    Rem(f32),
    /// Font Size relative to the size of the viewport width.
    Vw(f32),
    /// Font Size relative to the size of the viewport height.
    Vh(f32),
    /// Font Size relative to the smaller of the viewport width and viewport height.
    VMin(f32),
    /// Font Size relative to the larger of the viewport width and viewport height.
    VMax(f32),
    /// Calculated font size
    Calc(fn(FontCalc) -> f32),
}

impl PartialEq for FontSize {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Px(l0), Self::Px(r0)) => l0 == r0,
            (Self::Rem(l0), Self::Rem(r0)) => l0 == r0,
            (Self::Vw(l0), Self::Vw(r0)) => l0 == r0,
            (Self::Vh(l0), Self::Vh(r0)) => l0 == r0,
            (Self::VMin(l0), Self::VMin(r0)) => l0 == r0,
            (Self::VMax(l0), Self::VMax(r0)) => l0 == r0,
            _ => false,
        }
    }
}

/// Calculated font size
#[derive(Copy, Clone)]
pub struct FontCalc {
    viewport_size: Vec2,
    default_font_size: f32,
}

impl FontCalc {
    /// Size relative to default font size
    pub fn rem(&self) -> f32 {
        self.default_font_size
    }

    /// Viewport width
    pub fn vw(&self) -> f32 {
        self.viewport_size.x
    }

    /// Viewport height
    pub fn vh(&self) -> f32 {
        self.viewport_size.y
    }

    /// Minimum of viewport width and height
    pub fn vmin(&self) -> f32 {
        self.viewport_size.min_element()
    }

    /// Maximum of viewport width and height
    pub fn vmax(&self) -> f32 {
        self.viewport_size.max_element()
    }
}

impl FontSize {
    /// Evaluate the font size to a value in logical pixels
    pub fn eval(
        self,
        // Viewport size in logical pixels
        viewport_size: Vec2,
        // Default font size in logical pixels
        default_font_size: f32,
    ) -> f32 {
        match self {
            FontSize::Px(s) => s,
            FontSize::Rem(s) => default_font_size * s,
            FontSize::Vw(s) => viewport_size.x * s,
            FontSize::Vh(s) => viewport_size.y * s,
            FontSize::VMin(s) => viewport_size.min_element() * s,
            FontSize::VMax(s) => viewport_size.max_element() * s,
            FontSize::Calc(f) => f(FontCalc {
                viewport_size,
                default_font_size,
            }),
        }
    }
}

impl Default for FontSize {
    fn default() -> Self {
        Self::Px(20.)
    }
}

impl InheritableTextStyle for FontSize {
    type Inherited = FontSize;

    fn to_inherited(&self) -> InheritedTextStyle<Self::Inherited> {
        InheritedTextStyle(*self)
    }
}

/// Specifies the height of each line of text for `Text` and `Text2d`
///
/// Default is 1.2x the font size
#[derive(Debug, Clone, Copy, PartialEq, Reflect, Component)]
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

impl InheritableTextStyle for LineHeight {
    type Inherited = LineHeight;

    fn to_inherited(&self) -> InheritedTextStyle<Self::Inherited> {
        InheritedTextStyle(*self)
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

impl InheritableTextStyle for TextColor {
    type Inherited = TextColor;

    fn to_inherited(&self) -> InheritedTextStyle<Self::Inherited> {
        InheritedTextStyle(*self)
    }
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
#[derive(
    Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect, Serialize, Deserialize,
)]
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

impl InheritableTextStyle for FontSmoothing {
    type Inherited = FontSmoothing;

    fn to_inherited(&self) -> InheritedTextStyle<Self::Inherited> {
        InheritedTextStyle(*self)
    }
}

/// Text root
#[derive(Component, PartialEq)]
pub struct TextRoot(pub SmallVec<[Entity; 1]>);

/// Update text roots
pub fn update_text_roots<T: Component>(
    mut parents: Local<Vec<Entity>>,
    mut spans: Local<Vec<Entity>>,
    mut commands: Commands,
    mut text_node_query: Query<(
        Entity,
        Option<&ChildOf>,
        Option<&mut TextRoot>,
        Has<Children>,
        Ref<T>,
        Ref<ComputedTextStyle>,
        Ref<ComputedFontSize>,
    )>,
    children_query: Query<(
        Option<&Children>,
        Ref<T>,
        Ref<ComputedTextStyle>,
        Ref<ComputedFontSize>,
    )>,
) {
    for (entity, maybe_child_of, maybe_text_root, has_children, text, style, font_size) in
        text_node_query.iter_mut()
    {
        if maybe_child_of.is_none_or(|parent| !children_query.contains(parent.get())) {
            // Either the text entity is an orphan, or its parent is not a text entity. It must be a root text entity.
            if has_children {
                parents.push(entity);
            } else {
                let new_text_root = TextRoot(smallvec::smallvec![entity]);
                if let Some(mut text_root) = maybe_text_root {
                    text_root.set_if_neq(new_text_root);
                    if text.is_changed() || style.is_changed() || font_size.is_changed() {
                        text_root.set_changed();
                    }
                } else {
                    commands.entity(entity).insert(new_text_root);
                }
            }
        } else if maybe_text_root.is_some() {
            // Not a root. Remove `TextRoot` component, if present.
            commands.entity(entity).remove::<TextRoot>();
        }
    }

    for root_entity in parents.drain(..) {
        spans.clear();
        let mut changed = false;

        fn walk_text_descendants<T: Component>(
            target: Entity,
            query: &Query<(
                Option<&Children>,
                Ref<T>,
                Ref<ComputedTextStyle>,
                Ref<ComputedFontSize>,
            )>,
            spans: &mut Vec<Entity>,
            changed: &mut bool,
        ) {
            spans.push(target);
            if let Ok((children, text, style, size)) = query.get(target) {
                *changed |= text.is_changed() || style.is_changed() || size.is_changed();
                if let Some(children) = children {
                    for child in children {
                        walk_text_descendants(*child, query, spans, changed);
                    }
                }
            }
        }

        walk_text_descendants(root_entity, &children_query, &mut spans, &mut changed);

        if let Ok((_, _, Some(mut text_root), ..)) = text_node_query.get_mut(root_entity) {
            if text_root.0.as_slice() != spans.as_slice() {
                text_root.0.clear();
                text_root.0.extend(spans.iter().copied());
            }
            if changed {
                text_root.set_changed();
            }
        } else {
            commands
                .entity(root_entity)
                .insert(TextRoot(SmallVec::from_slice(&spans)));
        }
    }
}

/// Final font size
#[derive(Component, Debug, Copy, Clone, PartialEq, Deref, DerefMut, Default)]
pub struct ComputedFontSize(pub f32);
