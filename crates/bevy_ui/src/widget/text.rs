use crate::{stack::UiNodeToView, ContentSize, Measure, Node, UiLayouts};
use bevy_asset::Assets;
use bevy_ecs::{
    prelude::{Component, DetectChanges, Entity},
    query::With,
    reflect::ReflectComponent,
    system::{Query, Res, ResMut},
    world::Ref,
};
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_text::{
    Font, FontAtlasSet, FontAtlasWarning, Text, TextError, TextLayoutInfo, TextMeasureInfo,
    TextPipeline, TextSettings, YAxisOrientation,
};
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
            AvailableSpace::Definite(x) => x.clamp(
                self.info.min_width_content_size.x,
                self.info.max_width_content_size.x,
            ),
            AvailableSpace::MinContent => self.info.min_width_content_size.x,
            AvailableSpace::MaxContent => self.info.max_width_content_size.x,
        });

        height
            .map_or_else(
                || match available_width {
                    AvailableSpace::Definite(_) => self.info.compute_size(Vec2::new(x, f32::MAX)),
                    AvailableSpace::MinContent => Vec2::new(x, self.info.min_width_content_size.y),
                    AvailableSpace::MaxContent => Vec2::new(x, self.info.max_width_content_size.y),
                },
                |y| Vec2::new(x, y),
            )
            .ceil()
    }
}

/// Creates a `Measure` for text nodes that allows the UI to determine the appropriate amount of space
/// to provide for the text given the fonts, the text itself and the constraints of the layout.
pub fn measure_text_system(
    fonts: Res<Assets<Font>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(Entity, Ref<Text>, &mut ContentSize, &mut TextFlags), With<Node>>,
    uinode_map: Res<UiNodeToView>,
    ui_layouts: Res<UiLayouts>,
) {
    for (text_uinode, text, mut content_size, mut text_flags) in text_query.iter_mut() {
        if let Some(layout) = uinode_map
            .get(&text_uinode)
            .and_then(|entity| ui_layouts.get(entity))
        {
            if text.is_changed() || text_flags.needs_new_measure_func || layout.scale_factor_changed
            {
                match text_pipeline.create_text_measure(
                    &fonts,
                    &text.sections,
                    layout.context.combined_scale_factor,
                    text.alignment,
                    text.linebreak_behavior,
                ) {
                    Ok(measure) => {
                        content_size.set(TextMeasure { info: measure });

                        // Text measure func created succesfully, so set `TextFlags` to schedule a recompute
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
        }
    }
}

/// Updates the layout and size information whenever the text or style is changed.
/// This information is computed by the `TextPipeline` on insertion, then stored.
///
/// ## World Resources
///
/// [`ResMut<Assets<Image>>`](Assets<Image>) -- This system only adds new [`Image`] assets.
/// It does not modify or observe existing ones.
#[allow(clippy::too_many_arguments)]
pub fn text_system(
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    text_settings: Res<TextSettings>,
    mut font_atlas_warning: ResMut<FontAtlasWarning>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(
        Entity,
        Ref<Node>,
        &Text,
        &mut TextLayoutInfo,
        &mut TextFlags,
    )>,
    uinode_map: Res<UiNodeToView>,
    ui_layouts: Res<UiLayouts>,
) {
    for (text_uinode, node, text, mut text_layout_info, mut text_flags) in text_query.iter_mut() {
        if let Some(layout) = uinode_map
            .get(&text_uinode)
            .and_then(|entity| ui_layouts.get(entity))
        {
            if !text_flags.needs_new_measure_func
                && node.is_changed()
                || text_flags.needs_recompute
                || layout.scale_factor_changed 
            {
                match text_pipeline.queue_text(
                    &fonts,
                    &text.sections,
                    layout.context.combined_scale_factor,
                    text.alignment,
                    text.linebreak_behavior,
                    node.physical_size(layout.context.combined_scale_factor),
                    &mut font_atlas_set_storage,
                    &mut texture_atlases,
                    &mut textures,
                    &text_settings,
                    &mut font_atlas_warning,
                    YAxisOrientation::TopToBottom,
                ) {
                    Err(TextError::NoSuchFont) => {
                        // There was an error processing the text layout, try again next frame
                        text_flags.needs_recompute = true;
                    }
                    Err(e @ TextError::FailedToAddGlyph(_)) => {
                        panic!("Fatal error when processing text: {e}.");
                    }
                    Ok(info) => {
                        *text_layout_info = info;
                        text_flags.needs_recompute = false;
                    }
                }
            }
        }
    }
}
