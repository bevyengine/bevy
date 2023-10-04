use crate::{ContentSize, FixedMeasure, Measure, Node, UiScale};
use bevy_asset::Assets;
use bevy_ecs::{
    prelude::{Component, DetectChanges},
    query::With,
    reflect::ReflectComponent,
    system::{Local, Query, Res, ResMut},
    world::{Mut, Ref},
};
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_text::{
    scale_value, BreakLineOn, Font, FontAtlasSets, FontAtlasWarning, Text, TextError,
    TextLayoutInfo, TextMeasureInfo, TextPipeline, TextSettings, YAxisOrientation,
};
use bevy_window::{PrimaryWindow, Window};
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
    ) -> Vec2 {
        let x = width.unwrap_or_else(|| match available_width {
            AvailableSpace::Definite(x) => x.clamp(self.info.min.x, self.info.max.x),
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
    scale_factor: f64,
    text: Ref<Text>,
    mut content_size: Mut<ContentSize>,
    mut text_flags: Mut<TextFlags>,
) {
    match TextMeasureInfo::from_text(&text, fonts, scale_factor) {
        Ok(measure) => {
            if text.linebreak_behavior == BreakLineOn::NoWrap {
                content_size.set(FixedMeasure { size: measure.max });
            } else {
                content_size.set(TextMeasure { info: measure });
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
/// * All measures are regenerated if the primary window's scale factor or [`UiScale`] is changed.
/// * Changes that only modify the colors of a `Text` do not require a new `Measure`. This system
/// is only able to detect that a `Text` component has changed and will regenerate the `Measure` on
/// color changes. This can be expensive, particularly for large blocks of text, and the [`bypass_change_detection`](bevy_ecs::change_detection::DetectChangesMut::bypass_change_detection)
/// method should be called when only changing the `Text`'s colors.
pub fn measure_text_system(
    mut last_scale_factor: Local<f64>,
    fonts: Res<Assets<Font>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut text_query: Query<(Ref<Text>, &mut ContentSize, &mut TextFlags), With<Node>>,
) {
    let window_scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.);

    let scale_factor = ui_scale.0 * window_scale_factor;

    #[allow(clippy::float_cmp)]
    if *last_scale_factor == scale_factor {
        // scale factor unchanged, only create new measure funcs for modified text
        for (text, content_size, text_flags) in &mut text_query {
            if text.is_changed() || text_flags.needs_new_measure_func || content_size.is_added() {
                create_text_measure(&fonts, scale_factor, text, content_size, text_flags);
            }
        }
    } else {
        // scale factor changed, create new measure funcs for all text
        *last_scale_factor = scale_factor;

        for (text, content_size, text_flags) in &mut text_query {
            create_text_measure(&fonts, scale_factor, text, content_size, text_flags);
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn queue_text(
    fonts: &Assets<Font>,
    text_pipeline: &mut TextPipeline,
    font_atlas_warning: &mut FontAtlasWarning,
    font_atlas_sets: &mut FontAtlasSets,
    texture_atlases: &mut Assets<TextureAtlas>,
    textures: &mut Assets<Image>,
    text_settings: &TextSettings,
    scale_factor: f64,
    inverse_scale_factor: f64,
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
                (node.unrounded_size.x as f64 * scale_factor) as f32,
                (node.unrounded_size.y as f64 * scale_factor) as f32,
            )
        };

        match text_pipeline.queue_text(
            fonts,
            &text.sections,
            scale_factor,
            text.alignment,
            text.linebreak_behavior,
            physical_node_size,
            font_atlas_sets,
            texture_atlases,
            textures,
            text_settings,
            font_atlas_warning,
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
/// It does not modify or observe existing ones.
#[allow(clippy::too_many_arguments)]
pub fn text_system(
    mut textures: ResMut<Assets<Image>>,
    mut last_scale_factor: Local<f64>,
    fonts: Res<Assets<Font>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    text_settings: Res<TextSettings>,
    mut font_atlas_warning: ResMut<FontAtlasWarning>,
    ui_scale: Res<UiScale>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_sets: ResMut<FontAtlasSets>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(Ref<Node>, &Text, &mut TextLayoutInfo, &mut TextFlags)>,
) {
    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let window_scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.);

    let scale_factor = ui_scale.0 * window_scale_factor;
    let inverse_scale_factor = scale_factor.recip();
    if *last_scale_factor == scale_factor {
        // Scale factor unchanged, only recompute text for modified text nodes
        for (node, text, text_layout_info, text_flags) in &mut text_query {
            if node.is_changed() || text_flags.needs_recompute {
                queue_text(
                    &fonts,
                    &mut text_pipeline,
                    &mut font_atlas_warning,
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
    } else {
        // Scale factor changed, recompute text for all text nodes
        *last_scale_factor = scale_factor;

        for (node, text, text_layout_info, text_flags) in &mut text_query {
            queue_text(
                &fonts,
                &mut text_pipeline,
                &mut font_atlas_warning,
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
}
