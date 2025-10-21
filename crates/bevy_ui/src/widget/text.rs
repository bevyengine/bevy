use crate::{
    ComputedNode, ComputedUiRenderTargetInfo, ContentSize, FixedMeasure, Measure, MeasureArgs,
    Node, NodeMeasure,
};
use bevy_asset::Assets;
use bevy_color::{Color, LinearRgba};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    query::With,
    reflect::ReflectComponent,
    system::{Query, Res, ResMut},
    world::{Mut, Ref},
};
use bevy_image::prelude::*;
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_text::{
    build_layout_from_text_sections, build_text_layout_info, ComputedTextBlock, ComputedTextLayout,
    Font, FontAtlasSet, FontCx, LayoutCx, LineBreak, ScaleCx, TextBounds, TextColor, TextError,
    TextFont, TextHead, TextLayout, TextLayoutInfo, TextReader, TextSectionStyle, TextSpanAccess,
    TextWriter,
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
#[require(Node, TextLayout, TextFont, TextColor, TextNodeFlags, ContentSize)]
pub struct Text(pub String);

impl Text {
    /// Makes a new text component.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl TextHead for Text {}

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

/// Data for `TextMeasure`
pub struct TextMeasureInfo {
    // pub min: Vec2,
    // pub max: Vec2,
    pub entity: Entity,
}

impl TextMeasureInfo {
    /// Computes the size of the text area within the provided bounds.
    pub fn compute_size(&mut self, bounds: TextBounds, layout: &mut ComputedTextLayout) -> Vec2 {
        // Note that this arbitrarily adjusts the buffer layout. We assume the buffer is always 'refreshed'
        // whenever a canonical state is required.
        layout.break_all_lines(bounds.width);
        Vec2::new(layout.width(), layout.height())
    }
}

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
    fn measure(&mut self, measure_args: MeasureArgs, style: &taffy::Style) -> Vec2 {
        let MeasureArgs {
            width,
            height,
            available_width,
            available_height,
            maybe_text_layout,
            ..
        } = measure_args;

        let Some(text_layout) = maybe_text_layout else {
            error!("text measure failed, buffer is missing");
            return Vec2::ZERO;
        };

        let max = self.info.compute_size(TextBounds::default(), text_layout);

        let min = self
            .info
            .compute_size(TextBounds::new_horizontal(0.), text_layout);

        let x = width.unwrap_or_else(|| match available_width {
            AvailableSpace::Definite(x) => {
                self.info
                    .compute_size(TextBounds::new_horizontal(x), text_layout)
                    .x
            }
            AvailableSpace::MinContent => {
                self.info
                    .compute_size(TextBounds::new_horizontal(0.), text_layout)
                    .x
            }
            AvailableSpace::MaxContent => {
                self.info.compute_size(TextBounds::default(), text_layout).x
            }
        });

        height
            .map_or_else(
                || match available_height {
                    AvailableSpace::Definite(y) => (x, y.min(min.y)).into(),
                    AvailableSpace::MinContent => max,
                    AvailableSpace::MaxContent => min,
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
pub fn prepare_text_layout_system(
    mut font_cx: ResMut<FontCx>,
    mut layout_cx: ResMut<LayoutCx>,

    mut text_query: Query<
        (
            Entity,
            Ref<TextLayout>,
            &mut ContentSize,
            &mut TextNodeFlags,
            Ref<ComputedTextBlock>,
            &mut ComputedTextLayout,
            Ref<ComputedUiRenderTargetInfo>,
            &ComputedNode,
        ),
        With<Node>,
    >,
    mut text_reader: TextUiReader,
) {
    for (
        entity,
        block,
        mut content_size,
        text_flags,
        computed_block,
        mut computed_layout,
        computed_target,
        computed_node,
    ) in &mut text_query
    {
        // Note: the ComputedTextBlock::needs_rerender bool is cleared in create_text_measure().
        // 1e-5 epsilon to ignore tiny scale factor float errors
        if !(1e-5
            < (computed_target.scale_factor() - computed_node.inverse_scale_factor.recip()).abs()
            || computed_block.is_changed()
            || text_flags.needs_measure_fn
            || content_size.is_added())
        {
            continue;
        }

        let mut text_sections: Vec<&str> = Vec::new();
        let mut text_section_styles: Vec<TextSectionStyle<LinearRgba>> = Vec::new();
        for (_, _, text, font, color) in text_reader.iter(entity) {
            text_sections.push(text);
            text_section_styles.push(TextSectionStyle::new(
                font.font.as_str(),
                font.font_size,
                font.line_height,
                color.to_linear(),
            ));
        }

        build_layout_from_text_sections(
            &mut computed_layout.0,
            &mut font_cx.0,
            &mut layout_cx.0,
            text_sections.iter().copied(),
            text_section_styles.iter().copied(),
            computed_target.scale_factor,
            block.linebreak,
        );

        content_size.set(NodeMeasure::Text(TextMeasure {
            info: TextMeasureInfo {
                // min: (),
                // max: (),
                entity,
            },
        }));

        // match text_pipeline.create_text_measure(
        //     entity,
        //     fonts,
        //     spans,
        //     scale_factor,
        //     &block,
        //     computed_layout.as_mut(),
        // ) {
        //     Ok(measure) => {
        //         if block.linebreak == LineBreak::NoWrap {
        //             content_size.set(NodeMeasure::Fixed(FixedMeasure { size: measure.max }));
        //         } else {
        //             content_size.set(NodeMeasure::Text(TextMeasure { info: measure }));
        //         }

        //         // Text measure func created successfully, so set `TextNodeFlags` to schedule a recompute
        //         text_flags.needs_measure_fn = false;
        //         text_flags.needs_recompute = true;
        //     }
        //     Err(TextError::NoSuchFont) => {
        //         // Try again next frame
        //         text_flags.needs_measure_fn = true;
        //     }
        //     Err(e @ (TextError::FailedToAddGlyph(_) | TextError::FailedToGetGlyphImage)) => {
        //         panic!("Fatal error when processing text: {e}.");
        //     }
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
pub fn update_text_system(
    mut textures: ResMut<Assets<Image>>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut font_atlas_set: ResMut<FontAtlasSet>,
    mut text_query: Query<(
        Ref<ComputedNode>,
        &TextLayout,
        &mut TextLayoutInfo,
        &mut TextNodeFlags,
        &mut ComputedTextLayout,
    )>,
    mut scale_cx: ResMut<ScaleCx>,
) {
    for (node, block, mut text_layout_info, text_flags, mut layout) in &mut text_query {
        if node.is_changed() || text_flags.needs_recompute {
            *text_layout_info = build_text_layout_info(
                &mut layout.0,
                Some(node.size.x).filter(|_| block.linebreak != LineBreak::NoWrap),
                block.justify.into(),
                &mut scale_cx,
                &mut font_atlas_set,
                &mut texture_atlases,
                &mut textures,
                bevy_text::FontSmoothing::AntiAliased,
            );
        }
    }
}
