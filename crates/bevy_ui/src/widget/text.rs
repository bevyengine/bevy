use crate::{
    ContentSize, DefaultUiCamera, FixedMeasure, Measure, Node, NodeMeasure, TargetCamera, UiScale,
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
    scale_value, BreakLineOn, Font, FontAtlasSets, Text, TextError, TextLayoutInfo,
    TextMeasureInfo, TextPipeline, TextSettings, YAxisOrientation,
};
use bevy_utils::Entry;
use taffy::style::AvailableSpace;

/// Text system flags
///
/// Used internally by [`measure_text_system`] and [`text_system`] to schedule text for processing.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct TextFlags {
    /// If set a new measure function for the text node will be created
    needs_new_measure_func: bool,
    /// If set the text will be recomputed
    needs_recompute: bool,
}

impl Default for TextFlags {
    fn default() -> Self {
        Self {
            needs_new_measure_func: true,
            needs_recompute: true,
        }
    }
}

#[derive(Clone)]
pub struct TextMeasure {
    pub info: TextMeasureInfo,
}

impl Measure for TextMeasure {
    fn measure(
        &self,
        width: Option<f32>,
        height: Option<f32>,
        available_width: AvailableSpace,
        _available_height: AvailableSpace,
        _style: &taffy::Style,
    ) -> Vec2 {
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
                    AvailableSpace::Definite(_) => self.info.compute_size(Vec2::new(x, f32::MAX)),
                    AvailableSpace::MinContent => Vec2::new(x, self.info.min.y),
                    AvailableSpace::MaxContent => Vec2::new(x, self.info.max.y),
                },
                |y| Vec2::new(x, y),
            )
            .ceil()
    }
}

#[inline]
fn create_text_measure(
    fonts: &Assets<Font>,
    scale_factor: f32,
    text: Ref<Text>,
    mut content_size: Mut<ContentSize>,
    mut text_flags: Mut<TextFlags>,
) {
    match TextMeasureInfo::from_text(&text, fonts, scale_factor) {
        Ok(measure) => {
            if text.linebreak_behavior == BreakLineOn::NoWrap {
                content_size.set(NodeMeasure::Fixed(FixedMeasure { size: measure.max }));
            } else {
                content_size.set(NodeMeasure::Text(TextMeasure { info: measure }));
            }

            // Text measure func created successfully, so set `TextFlags` to schedule a recompute
            text_flags.needs_new_measure_func = false;
            text_flags.needs_recompute = true;
        }
        Err(TextError::NoSuchFont) => {
            // Try again next frame
            text_flags.needs_new_measure_func = true;
        }
        Err(e @ TextError::FailedToAddGlyph(_)) => {
            panic!("Fatal error when processing text: {e}.");
        }
    };
}

/// Generates a new [`Measure`] for a text node on changes to its [`Text`] component.
/// A `Measure` is used by the UI's layout algorithm to determine the appropriate amount of space
/// to provide for the text given the fonts, the text itself and the constraints of the layout.
///
/// * Measures are regenerated if the target camera's scale factor (or primary window if no specific target) or [`UiScale`] is changed.
/// * Changes that only modify the colors of a `Text` do not require a new `Measure`. This system
/// is only able to detect that a `Text` component has changed and will regenerate the `Measure` on
/// color changes. This can be expensive, particularly for large blocks of text, and the [`bypass_change_detection`](bevy_ecs::change_detection::DetectChangesMut::bypass_change_detection)
/// method should be called when only changing the `Text`'s colors.
pub fn measure_text_system(
    mut last_scale_factors: Local<EntityHashMap<f32>>,
    fonts: Res<Assets<Font>>,
    camera_query: Query<(Entity, &Camera)>,
    default_ui_camera: DefaultUiCamera,
    ui_scale: Res<UiScale>,
    mut text_query: Query<
        (
            Ref<Text>,
            &mut ContentSize,
            &mut TextFlags,
            Option<&TargetCamera>,
        ),
        With<Node>,
    >,
) {
    let mut scale_factors: EntityHashMap<f32> = EntityHashMap::default();

    for (text, content_size, text_flags, camera) in &mut text_query {
        let Some(camera_entity) = camera.map(TargetCamera::entity).or(default_ui_camera.get())
        else {
            continue;
        };
        let scale_factor = match scale_factors.entry(camera_entity) {
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
        if last_scale_factors.get(&camera_entity) != Some(&scale_factor)
            || text.is_changed()
            || text_flags.needs_new_measure_func
            || content_size.is_added()
        {
            create_text_measure(&fonts, scale_factor, text, content_size, text_flags);
        }
    }
    *last_scale_factors = scale_factors;
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn queue_text(
    fonts: &Assets<Font>,
    text_pipeline: &mut TextPipeline,
    font_atlas_sets: &mut FontAtlasSets,
    texture_atlases: &mut Assets<TextureAtlasLayout>,
    textures: &mut Assets<Image>,
    text_settings: &TextSettings,
    scale_factor: f32,
    inverse_scale_factor: f32,
    text: &Text,
    node: Ref<Node>,
    mut text_flags: Mut<TextFlags>,
    mut text_layout_info: Mut<TextLayoutInfo>,
) {
    // Skip the text node if it is waiting for a new measure func
    if !text_flags.needs_new_measure_func {
        let physical_node_size = if text.linebreak_behavior == BreakLineOn::NoWrap {
            // With `NoWrap` set, no constraints are placed on the width of the text.
            Vec2::splat(f32::INFINITY)
        } else {
            // `scale_factor` is already multiplied by `UiScale`
            Vec2::new(
                node.unrounded_size.x * scale_factor,
                node.unrounded_size.y * scale_factor,
            )
        };

        match text_pipeline.queue_text(
            fonts,
            &text.sections,
            scale_factor,
            text.justify,
            text.linebreak_behavior,
            physical_node_size,
            font_atlas_sets,
            texture_atlases,
            textures,
            text_settings,
            YAxisOrientation::TopToBottom,
        ) {
            Err(TextError::NoSuchFont) => {
                // There was an error processing the text layout, try again next frame
                text_flags.needs_recompute = true;
            }
            Err(e @ TextError::FailedToAddGlyph(_)) => {
                panic!("Fatal error when processing text: {e}.");
            }
            Ok(mut info) => {
                info.logical_size.x = scale_value(info.logical_size.x, inverse_scale_factor);
                info.logical_size.y = scale_value(info.logical_size.y, inverse_scale_factor);
                *text_layout_info = info;
                text_flags.needs_recompute = false;
            }
        }
    }
}

/// Updates the layout and size information for a UI text node on changes to the size value of its [`Node`] component,
/// or when the `needs_recompute` field of [`TextFlags`] is set to true.
/// This information is computed by the [`TextPipeline`] and then stored in [`TextLayoutInfo`].
///
/// ## World Resources
///
/// [`ResMut<Assets<Image>>`](Assets<Image>) -- This system only adds new [`Image`] assets.
/// It does not modify or observe existing ones. The exception is when adding new glyphs to a [`bevy_text::FontAtlas`].
#[allow(clippy::too_many_arguments)]
pub fn text_system(
    mut textures: ResMut<Assets<Image>>,
    mut last_scale_factors: Local<EntityHashMap<f32>>,
    fonts: Res<Assets<Font>>,
    camera_query: Query<(Entity, &Camera)>,
    default_ui_camera: DefaultUiCamera,
    text_settings: Res<TextSettings>,
    ui_scale: Res<UiScale>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut font_atlas_sets: ResMut<FontAtlasSets>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(
        Ref<Node>,
        &Text,
        &mut TextLayoutInfo,
        &mut TextFlags,
        Option<&TargetCamera>,
    )>,
) {
    let mut scale_factors: EntityHashMap<f32> = EntityHashMap::default();

    for (node, text, text_layout_info, text_flags, camera) in &mut text_query {
        let Some(camera_entity) = camera.map(TargetCamera::entity).or(default_ui_camera.get())
        else {
            continue;
        };
        let scale_factor = match scale_factors.entry(camera_entity) {
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
                &text_settings,
                scale_factor,
                inverse_scale_factor,
                text,
                node,
                text_flags,
                text_layout_info,
            );
        }
    }
    *last_scale_factors = scale_factors;
}
