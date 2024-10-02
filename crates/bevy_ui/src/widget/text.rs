use crate::{
    ContentSize, DefaultUiCamera, FixedMeasure, Measure, MeasureArgs, Node, NodeMeasure,
    TargetCamera, UiScale,
};
use bevy_asset::Assets;
use bevy_ecs::{
    entity::{Entity, EntityHashMap},
    prelude::{Component, DetectChanges},
    query::With,
    reflect::ReflectComponent,
    system::{Local, Query, Res, ResMut},
    world::{Mut, Ref},
};
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{camera::Camera, texture::Image};
use bevy_sprite::TextureAtlasLayout;
use bevy_text::{
    scale_value, CosmicBuffer, CosmicFontSystem, Font, FontAtlasSets, JustifyText, LineBreak,
    SwashCache, Text, TextBounds, TextError, TextLayoutInfo, TextMeasureInfo, TextPipeline,
    YAxisOrientation,
};
use bevy_utils::{tracing::error, Entry};
use taffy::style::AvailableSpace;

/// UI text system flags.
///
/// Used internally by [`measure_text_system`] and [`text_system`] to schedule text for processing.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default, Debug)]
pub struct TextNodeFlags {
    /// If set a new measure function for the text node will be created.
    needs_measure_fn: bool,
    /// If set the text will be recomputed.
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
/// Adding `TextNEW` to an entity will pull in required components for setting up a UI text node.
///
/// The string in this component is the first 'text span' in a hierarchy of text spans that are collected into
/// a [`TextBlock`]. See [`TextSpan`] for the component used by children of entities with `TextNEW`.
///
/// Note that [`Transform`] on this entity is managed automatically by the UI layout system.
/*
```
# use bevy_ecs::World;
# use bevy_text::{JustifyText, TextBlock};
# use bevy_ui::Text;
#
# let mut world = World::default();
#
// Basic usage.
world.spawn(Text::new("hello world!"));

// With text justification.
world.spawn((
    Text::new("hello world\nand bevy!"),
    TextBlock::new_with_justify(JustifyText::Center)
));
```
*/
#[derive(Component, Debug, Default, Clone, Deref, DerefMut, Reflect)]
#[reflect(Component, Default, Debug)]
#[require(
    TextBlock,
    TextStyle,
    TextNodeFlags,
    Node,
    Style, // TODO: Remove when Node uses required components.
    ContentSize, // TODO: Remove when Node uses required components.
    FocusPolicy, // TODO: Remove when Node uses required components.
    ZIndex, // TODO: Remove when Node uses required components.
    BackgroundColor, // TODO: Remove when Node uses required components.
    Visibility, // TODO: Remove when Node uses required components.
    Transform // TODO: Remove when Node uses required components.
)]
pub struct TextNEW(pub String);

impl TextNEW {
    /// Makes a new text component.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl From<&str> for TextNEW {
    fn from(value: &str) -> Self {
        Self(String::from(value))
    }
}

impl From<String> for TextNEW {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// A span of UI text in a tree of spans under an entity with [`Text`].
///
/// Spans are collected in hierarchy traversal order into a [`ComputedTextBlock`] for layout.
/*
```
# use bevy_asset::Handle;
# use bevy_color::Color;
# use bevy_color::palettes::basic::{RED, BLUE};
# use bevy_ecs::World;
# use bevy_text::{Font, TextStyle, TextSection};
# use bevy_ui::{Text, TextSpan};
#
# let font_handle: Handle<Font> = Default::default();
# let mut world = World::default();
#
world.spawn((
    Text::new("Hello, "),
    TextStyle {
        font: font_handle.clone().into(),
        font_size: 60.0,
        color: BLUE.into(),
    }
))
.with_child((
    TextSpan::new("World!"),
    TextStyle {
        font: font_handle.into(),
        font_size: 60.0,
        color: RED.into(),
    }
));
```
*/
#[derive(Component, Debug, Default, Clone, Deref, DerefMut, Reflect)]
#[reflect(Component, Default, Debug)]
#[require(TextStyle, GhostNode, Visibility = Visibility::Hidden)]
pub struct TextSpan(pub String);

impl TextSpanReader for TextSpan {
    fn read_span(&self) -> &str {
        self.as_str()
    }
}

/// Text measurement for UI layout. See [`NodeMeasure`].
pub struct TextMeasure {
    pub info: TextMeasureInfo,
}

impl TextMeasure {
    /// Checks if the cosmic text buffer is needed for measuring the text.
    pub fn needs_buffer(height: Option<f32>, available_width: AvailableSpace) -> bool {
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

#[allow(clippy::too_many_arguments)]
#[inline]
fn create_text_measure<'a>(
    entity: Entity,
    fonts: &Assets<Font>,
    scale_factor: f64,
    spans: impl Iterator<(&'a str, &'a TextStyle)>,
    block: Ref<TextBlock>,
    text_pipeline: &mut TextPipeline,
    mut content_size: Mut<ContentSize>,
    mut text_flags: Mut<TextNodeFlags>,
    mut computed: Mut<ComputedTextBlock>,
    font_system: &mut CosmicFontSystem,
) {
    match text_pipeline.create_text_measure(
        entity,
        fonts,
        spans,
        scale_factor,
        block,
        computed,
        font_system,
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
        Err(TextError::NoSuchFont) => {
            // Try again next frame
            text_flags.needs_measure_fn = true;
        }
        Err(e @ (TextError::FailedToAddGlyph(_) | TextError::FailedToGetGlyphImage(_))) => {
            panic!("Fatal error when processing text: {e}.");
        }
    };
}

/// Generates a new [`Measure`] for a text node on changes to its [`Text`] component.
///
/// A `Measure` is used by the UI's layout algorithm to determine the appropriate amount of space
/// to provide for the text given the fonts, the text itself and the constraints of the layout.
///
/// * Measures are regenerated if the target camera's scale factor (or primary window if no specific target) or [`UiScale`] is changed.
/// * Changes that only modify the colors of a `Text` do not require a new `Measure`. This system
///     is only able to detect that a `Text` component has changed and will regenerate the `Measure` on
///     color changes. This can be expensive, particularly for large blocks of text, and the [`bypass_change_detection`](bevy_ecs::change_detection::DetectChangesMut::bypass_change_detection)
///     method should be called when only changing the `Text`'s colors.
#[allow(clippy::too_many_arguments)]
pub fn measure_text_system(
    mut scale_factors_buffer: Local<EntityHashMap<f32>>,
    mut last_scale_factors: Local<EntityHashMap<f32>>,
    fonts: Res<Assets<Font>>,
    camera_query: Query<(Entity, &Camera)>,
    default_ui_camera: DefaultUiCamera,
    ui_scale: Res<UiScale>,
    mut text_query: Query<
        (
            Entity,
            Ref<Text>,
            Ref<TextStyle>,
            Ref<TextBlock>,
            &mut ContentSize,
            &mut TextNodeFlags,
            &mut ComputedTextBlock,
            Option<&TargetCamera>,
            Option<&Children>,
        ),
        With<Node>,
    >,
    mut spans: TextSpans<TextSpan>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut font_system: ResMut<CosmicFontSystem>,
) {
    scale_factors_buffer.clear();

    for (
        entity,
        text,
        text_style,
        block,
        content_size,
        text_flags,
        computed,
        maybe_camera,
        maybe_children,
    ) in &mut text_query
    {
        let Some(camera_entity) = maybe_camera
            .map(TargetCamera::entity)
            .or(default_ui_camera.get())
        else {
            continue;
        };
        let scale_factor = match scale_factors_buffer.entry(camera_entity) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(
                camera_query
                    .get(camera_entity)
                    .ok()
                    .and_then(|(_, c)| c.target_scaling_factor())
                    .unwrap_or(1.0)
                    * ui_scale.0,
            ),
        };
        // Note: the ComputedTextBlock::needs_rerender bool is cleared in create_text_measure().
        if last_scale_factors.get(&camera_entity) != Some(&scale_factor)
            || computed.needs_rerender()
            || text_flags.needs_measure_fn
            || content_size.is_added()
        {
            create_text_measure(
                entity,
                &fonts,
                scale_factor.into(),
                spans.iter_from_base(text.as_str(), text_style, maybe_children),
                block,
                &mut text_pipeline,
                content_size,
                text_flags,
                computed,
                &mut font_system,
            );
        }
    }
    core::mem::swap(&mut *last_scale_factors, &mut *scale_factors_buffer);
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn queue_text(
    fonts: &Assets<Font>,
    text_pipeline: &mut TextPipeline,
    font_atlas_sets: &mut FontAtlasSets,
    texture_atlases: &mut Assets<TextureAtlasLayout>,
    textures: &mut Assets<Image>,
    scale_factor: f32,
    inverse_scale_factor: f32,
    text: &Text,
    text_style: &TextStyle,
    block: &TextBlock,
    maybe_children: Option<&Children>,
    node: Ref<Node>,
    mut text_flags: Mut<TextNodeFlags>,
    text_layout_info: Mut<TextLayoutInfo>,
    computed: &mut ComputedTextBlock,
    spans: &mut TextSpans<TextSpan>,
    font_system: &mut CosmicFontSystem,
    swash_cache: &mut SwashCache,
) {
    // Skip the text node if it is waiting for a new measure func
    if text_flags.needs_measure_fn {
        return;
    }

    let physical_node_size = if text.linebreak == LineBreak::NoWrap {
        // With `NoWrap` set, no constraints are placed on the width of the text.
        TextBounds::UNBOUNDED
    } else {
        // `scale_factor` is already multiplied by `UiScale`
        TextBounds::new(
            node.unrounded_size.x * scale_factor,
            node.unrounded_size.y * scale_factor,
        )
    };

    let text_layout_info = text_layout_info.into_inner();
    match text_pipeline.queue_text(
        text_layout_info,
        fonts,
        spans.iter_from_base(text.as_str(), text_style, maybe_children),
        scale_factor.into(),
        block,
        physical_node_size,
        font_atlas_sets,
        texture_atlases,
        textures,
        YAxisOrientation::TopToBottom,
        computed,
        font_system,
        swash_cache,
    ) {
        Err(TextError::NoSuchFont) => {
            // There was an error processing the text layout, try again next frame
            text_flags.needs_recompute = true;
        }
        Err(e @ (TextError::FailedToAddGlyph(_) | TextError::FailedToGetGlyphImage(_))) => {
            panic!("Fatal error when processing text: {e}.");
        }
        Ok(()) => {
            text_layout_info.size.x = scale_value(text_layout_info.size.x, inverse_scale_factor);
            text_layout_info.size.y = scale_value(text_layout_info.size.y, inverse_scale_factor);
            text_flags.needs_recompute = false;
        }
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
#[allow(clippy::too_many_arguments)]
pub fn text_system(
    mut textures: ResMut<Assets<Image>>,
    mut scale_factors_buffer: Local<EntityHashMap<f32>>,
    mut last_scale_factors: Local<EntityHashMap<f32>>,
    fonts: Res<Assets<Font>>,
    camera_query: Query<(Entity, &Camera)>,
    default_ui_camera: DefaultUiCamera,
    ui_scale: Res<UiScale>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut font_atlas_sets: ResMut<FontAtlasSets>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(
        Ref<Node>,
        &Text,
        &TextStyle,
        &TextBlock,
        &mut TextLayoutInfo,
        &mut TextNodeFlags,
        &mut ComputedTextBlock,
        Option<&TargetCamera>,
        Option<&Children>,
    )>,
    mut spans: TextSpans<TextSpan>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut swash_cache: ResMut<SwashCache>,
) {
    scale_factors_buffer.clear();

    for (
        node,
        text,
        text_style,
        block,
        text_layout_info,
        text_flags,
        mut computed,
        maybe_camera,
        maybe_children,
    ) in &mut text_query
    {
        let Some(camera_entity) = maybe_camera
            .map(TargetCamera::entity)
            .or(default_ui_camera.get())
        else {
            continue;
        };
        let scale_factor = match scale_factors_buffer.entry(camera_entity) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(
                camera_query
                    .get(camera_entity)
                    .ok()
                    .and_then(|(_, c)| c.target_scaling_factor())
                    .unwrap_or(1.0)
                    * ui_scale.0,
            ),
        };
        let inverse_scale_factor = scale_factor.recip();

        if last_scale_factors.get(&camera_entity) != Some(&scale_factor)
            || node.is_changed()
            || text_flags.needs_recompute
        {
            queue_text(
                &fonts,
                &mut text_pipeline,
                &mut font_atlas_sets,
                &mut texture_atlases,
                &mut textures,
                scale_factor,
                inverse_scale_factor,
                text,
                text_style,
                block,
                maybe_children,
                node,
                text_flags,
                text_layout_info,
                computed.as_mut(),
                &mut spans,
                &mut font_system,
                &mut swash_cache,
            );
        }
    }
    core::mem::swap(&mut *last_scale_factors, &mut *scale_factors_buffer);
}
