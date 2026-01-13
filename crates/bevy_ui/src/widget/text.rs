use crate::{
    ComputedNode, ComputedUiRenderTargetInfo, ContentSize, FixedMeasure, Measure, MeasureArgs,
    Node, NodeMeasure,
};
use bevy_asset::Assets;
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    query::With,
    reflect::ReflectComponent,
    system::{Query, Res, ResMut},
    world::Ref,
};
use bevy_image::prelude::*;
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_text::{
    ComputedTextBlock, CosmicFontSystem, Font, FontAtlasSet, FontHinting, LineBreak, LineHeight,
    SwashCache, TextBounds, TextColor, TextError, TextFont, TextLayout, TextLayoutInfo,
    TextMeasureInfo, TextPipeline, TextReader, TextRoot, TextSpanAccess, TextWriter,
};
use taffy::style::AvailableSpace;
use tracing::error;

/// UI text system flags.
///
/// Used internally by [`measure_text_system`] and [`text_system`] to schedule text for processing.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct TextNodeFlags {
    /// If set then a new measure function for the text node will be created.
    needs_measure_fn: bool,
    /// If set then the text will be recomputed.
    needs_recompute: bool,
}

impl Default for TextNodeFlags {
    fn default() -> Self {
        Self {
            needs_measure_fn: true,
            needs_recompute: true,
        }
    }
}

/// The top-level UI text component.
///
/// Adding [`Text`] to an entity will pull in required components for setting up a UI text node.
///
/// The string in this component is the first 'text span' in a hierarchy of text spans that are collected into
/// a [`ComputedTextBlock`]. See [`TextSpan`](bevy_text::TextSpan) for the component used by children of entities with [`Text`].
///
/// Note that [`Transform`](bevy_transform::components::Transform) on this entity is managed automatically by the UI layout system.
///
///
/// ```
/// # use bevy_asset::Handle;
/// # use bevy_color::Color;
/// # use bevy_color::palettes::basic::BLUE;
/// # use bevy_ecs::world::World;
/// # use bevy_text::{Font, Justify, TextLayout, TextFont, TextColor, TextSpan};
/// # use bevy_ui::prelude::Text;
/// #
/// # let font_handle: Handle<Font> = Default::default();
/// # let mut world = World::default();
/// #
/// // Basic usage.
/// world.spawn(Text::new("hello world!"));
///
/// // With non-default style.
/// world.spawn((
///     Text::new("hello world!"),
///     TextFont {
///         font: font_handle.clone().into(),
///         font_size: 60.0,
///         ..Default::default()
///     },
///     TextColor(BLUE.into()),
/// ));
///
/// // With text justification.
/// world.spawn((
///     Text::new("hello world\nand bevy!"),
///     TextLayout::new_with_justify(Justify::Center)
/// ));
///
/// // With spans
/// world.spawn(Text::new("hello ")).with_children(|parent| {
///     parent.spawn(TextSpan::new("world"));
///     parent.spawn((TextSpan::new("!"), TextColor(BLUE.into())));
/// });
/// ```
#[derive(Component, Debug, Default, Clone, Deref, DerefMut, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(
    Node,
    TextLayout,
    TextFont,
    TextColor,
    LineHeight,
    TextNodeFlags,
    ContentSize,
    // Disable hinting.
    // UI text is normally pixel-aligned, but with hinting enabled sometimes the text bounds are miscalculated slightly.
    FontHinting::Disabled
)]
pub struct Text(pub String);

impl Text {
    /// Makes a new text component.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl TextRoot for Text {}

impl TextSpanAccess for Text {
    fn read_span(&self) -> &str {
        self.as_str()
    }
    fn write_span(&mut self) -> &mut String {
        &mut *self
    }
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self(String::from(value))
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Adds a shadow behind text
///
/// Use the `Text2dShadow` component for `Text2d` shadows
#[derive(Component, Copy, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, Clone, PartialEq)]
pub struct TextShadow {
    /// Shadow displacement in logical pixels
    /// With a value of zero the shadow will be hidden directly behind the text
    pub offset: Vec2,
    /// Color of the shadow
    pub color: Color,
}

impl Default for TextShadow {
    fn default() -> Self {
        Self {
            offset: Vec2::splat(4.),
            color: Color::linear_rgba(0., 0., 0., 0.75),
        }
    }
}

/// UI alias for [`TextReader`].
pub type TextUiReader<'w, 's> = TextReader<'w, 's, Text>;

/// UI alias for [`TextWriter`].
pub type TextUiWriter<'w, 's> = TextWriter<'w, 's, Text>;

/// Text measurement for UI layout. See [`NodeMeasure`].
pub struct TextMeasure {
    pub info: TextMeasureInfo,
}

impl TextMeasure {
    /// Checks if the cosmic text buffer is needed for measuring the text.
    #[inline]
    pub const fn needs_buffer(height: Option<f32>, available_width: AvailableSpace) -> bool {
        height.is_none() && matches!(available_width, AvailableSpace::Definite(_))
    }
}

impl Measure for TextMeasure {
    fn measure(&mut self, measure_args: MeasureArgs, _style: &taffy::Style) -> Vec2 {
        let MeasureArgs {
            width,
            height,
            available_width,
            buffer,
            font_system,
            ..
        } = measure_args;
        let x = width.unwrap_or_else(|| match available_width {
            AvailableSpace::Definite(x) => {
                // It is possible for the "min content width" to be larger than
                // the "max content width" when soft-wrapping right-aligned text
                // and possibly other situations.

                x.max(self.info.min.x).min(self.info.max.x)
            }
            AvailableSpace::MinContent => self.info.min.x,
            AvailableSpace::MaxContent => self.info.max.x,
        });

        height
            .map_or_else(
                || match available_width {
                    AvailableSpace::Definite(_) => {
                        if let Some(buffer) = buffer {
                            self.info.compute_size(
                                TextBounds::new_horizontal(x),
                                buffer,
                                font_system,
                            )
                        } else {
                            error!("text measure failed, buffer is missing");
                            Vec2::default()
                        }
                    }
                    AvailableSpace::MinContent => Vec2::new(x, self.info.min.y),
                    AvailableSpace::MaxContent => Vec2::new(x, self.info.max.y),
                },
                |y| Vec2::new(x, y),
            )
            .ceil()
    }
}

/// Generates a new [`Measure`] for a text node on changes to its [`Text`] component.
///
/// A `Measure` is used by the UI's layout algorithm to determine the appropriate amount of space
/// to provide for the text given the fonts, the text itself and the constraints of the layout.
///
/// * Measures are regenerated on changes to either [`ComputedTextBlock`] or [`ComputedUiRenderTargetInfo`].
/// * Changes that only modify the colors of a `Text` do not require a new `Measure`. This system
///   is only able to detect that a `Text` component has changed and will regenerate the `Measure` on
///   color changes. This can be expensive, particularly for large blocks of text, and the [`bypass_change_detection`](bevy_ecs::change_detection::DetectChangesMut::bypass_change_detection)
///   method should be called when only changing the `Text`'s colors.
pub fn measure_text_system(
    fonts: Res<Assets<Font>>,
    mut text_query: Query<
        (
            Entity,
            Ref<TextLayout>,
            &mut ContentSize,
            &mut TextNodeFlags,
            &mut ComputedTextBlock,
            Ref<ComputedUiRenderTargetInfo>,
            &ComputedNode,
            Ref<FontHinting>,
        ),
        With<Node>,
    >,
    mut text_reader: TextUiReader,
    mut text_pipeline: ResMut<TextPipeline>,
    mut font_system: ResMut<CosmicFontSystem>,
) {
    for (
        entity,
        block,
        mut content_size,
        mut text_flags,
        mut computed,
        computed_target,
        computed_node,
        hinting,
    ) in &mut text_query
    {
        // Note: the ComputedTextBlock::needs_rerender bool is cleared in create_text_measure().
        // 1e-5 epsilon to ignore tiny scale factor float errors
        if !(1e-5
            < (computed_target.scale_factor() - computed_node.inverse_scale_factor.recip()).abs()
            || computed.needs_rerender()
            || text_flags.needs_measure_fn
            || content_size.is_added()
            || hinting.is_changed())
        {
            continue;
        }

        match text_pipeline.create_text_measure(
            entity,
            fonts.as_ref(),
            text_reader.iter(entity),
            computed_target.scale_factor.into(),
            &block,
            computed.as_mut(),
            &mut font_system,
            *hinting,
        ) {
            Ok(measure) => {
                if block.linebreak == LineBreak::NoWrap {
                    content_size.set(NodeMeasure::Fixed(FixedMeasure { size: measure.max }));
                } else {
                    content_size.set(NodeMeasure::Text(TextMeasure { info: measure }));
                }

                // Text measure func created successfully, so set `TextNodeFlags` to schedule a recompute
                text_flags.needs_measure_fn = false;
                text_flags.needs_recompute = true;
            }
            Err(TextError::NoSuchFont | TextError::DegenerateScaleFactor) => {
                // Try again next frame
                text_flags.needs_measure_fn = true;
            }
            Err(
                e @ (TextError::FailedToAddGlyph(_)
                | TextError::FailedToGetGlyphImage(_)
                | TextError::MissingAtlasLayout
                | TextError::MissingAtlasTexture
                | TextError::InconsistentAtlasState),
            ) => {
                panic!("Fatal error when processing text: {e}.");
            }
        };
    }
}

/// Updates the layout and size information for a UI text node on changes to the size value of its [`Node`] component,
/// or when the `needs_recompute` field of [`TextNodeFlags`] is set to true.
/// This information is computed by the [`TextPipeline`] and then stored in [`TextLayoutInfo`].
///
/// ## World Resources
///
/// [`ResMut<Assets<Image>>`](Assets<Image>) -- This system only adds new [`Image`] assets.
/// It does not modify or observe existing ones. The exception is when adding new glyphs to a [`bevy_text::FontAtlas`].
pub fn text_system(
    mut textures: ResMut<Assets<Image>>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut font_atlas_set: ResMut<FontAtlasSet>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(
        Ref<ComputedNode>,
        &TextLayout,
        &mut TextLayoutInfo,
        &mut TextNodeFlags,
        &mut ComputedTextBlock,
    )>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut swash_cache: ResMut<SwashCache>,
) {
    for (node, block, mut text_layout_info, mut text_flags, mut computed) in &mut text_query {
        if node.is_changed() || text_flags.needs_recompute {
            // Skip the text node if it is waiting for a new measure func
            if text_flags.needs_measure_fn {
                continue;
            }

            let physical_node_size = if block.linebreak == LineBreak::NoWrap {
                // With `NoWrap` set, no constraints are placed on the width of the text.
                TextBounds::UNBOUNDED
            } else {
                // `scale_factor` is already multiplied by `UiScale`
                TextBounds::new(node.unrounded_size.x, node.unrounded_size.y)
            };

            match text_pipeline.update_text_layout_info(
                &mut text_layout_info,
                &mut font_atlas_set,
                &mut texture_atlases,
                &mut textures,
                &mut computed,
                &mut font_system,
                &mut swash_cache,
                physical_node_size,
                block.justify,
            ) {
                Err(TextError::NoSuchFont | TextError::DegenerateScaleFactor) => {
                    // There was an error processing the text layout, try again next frame
                    text_flags.needs_recompute = true;
                }
                Err(
                    e @ (TextError::FailedToAddGlyph(_)
                    | TextError::FailedToGetGlyphImage(_)
                    | TextError::MissingAtlasLayout
                    | TextError::MissingAtlasTexture
                    | TextError::InconsistentAtlasState),
                ) => {
                    panic!("Fatal error when processing text: {e}.");
                }
                Ok(()) => {
                    text_layout_info.scale_factor = node.inverse_scale_factor().recip();
                    text_layout_info.size *= node.inverse_scale_factor();
                    text_flags.needs_recompute = false;
                }
            }
        }
    }
}
