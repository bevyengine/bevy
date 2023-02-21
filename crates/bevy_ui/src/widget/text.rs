use crate::{IntrinsicSize, Node, UiScale, Measure};
use bevy_asset::Assets;
use bevy_ecs::{
    entity::Entity,
    query::{Changed, Or, With},
    system::{Local, ParamSet, Query, Res, ResMut, Resource},
};
use bevy_math::Vec2;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_text::{
    Font, FontAtlasSet, FontAtlasWarning, Text, TextError, TextLayoutInfo, TextPipeline,
    TextSettings, YAxisOrientation,
};
use bevy_window::{PrimaryWindow, Window};
use crate::measurement::AvailableSpace;

fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}

#[derive(Clone)]
pub struct TextMeasure {
    pub min_content: Vec2,
    pub max_content: Vec2,
}

impl Measure for TextMeasure {
    fn measure(
        &self,
        max_width: Option<f32>,
        max_height: Option<f32>,
        _: AvailableSpace,
        _: AvailableSpace,
    ) -> Vec2 {
        Vec2::new(
            match max_width {
                Some(width) => width,
                None => self.max_content.x,
            },
            match max_height {
                Some(height) => height,
                None => self.min_content.y,
            }
        )
        // taffy rounds the size down to the nearest pixel, which can cause the text to be cut off
        .ceil()
    }

    fn dyn_clone(&self) -> Box<dyn Measure> {
        Box::new(self.clone())
    }
}

#[derive(Resource, Default)]
pub struct TextQueue {
    pub ids: Vec<Entity>,
}

/// Creates a `Measure` for text nodes that allows the UI to determine the appropriate amount of space 
/// to provide for the text given the fonts, the text itself and the constraints of the layout.
pub fn measure_text_system(
    mut text_queue: ResMut<TextQueue>,
    mut queued_text_ids: Local<Vec<Entity>>,
    mut last_scale_factor: Local<f64>,
    fonts: Res<Assets<Font>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_queries: ParamSet<(
        Query<Entity, Changed<Text>>,
        Query<Entity, With<Text>>,
        Query<(&Text, &mut IntrinsicSize)>,
    )>,
) {
    let window_scale_factor = windows
    .get_single()
    .map(|window| window.resolution.scale_factor())
    .unwrap_or(1.);

    let scale_factor = ui_scale.scale * window_scale_factor;

    #[allow(clippy::float_cmp)]
    if *last_scale_factor == scale_factor {
        // Adds all entities where the text or the style has changed to the local queue
        for entity in text_queries.p0().iter() {
            queued_text_ids.push(entity);
        }
    } else {
        // If the scale factor has changed, queue all text
        for entity in text_queries.p1().iter() {
            queued_text_ids.push(entity);
        }
        *last_scale_factor = scale_factor;
    }

    if queued_text_ids.is_empty() {
        return;
    }

    let mut new_queue = Vec::new();
    let mut query = text_queries.p2();
    for entity in queued_text_ids.drain(..) {
        if let Ok((text, mut calculated_size)) = query.get_mut(entity) {
            match text_pipeline.compute_sections(&fonts, &text.sections, scale_factor) {
                Ok((sections, scaled_fonts)) => {
                    // computes the size of the text with the text wrapped after every word
                    let min = text_pipeline.compute_size(
                        &sections,
                        &scaled_fonts,
                        text.alignment,
                        text.linebreak_behaviour,
                        Vec2::new(0., f32::INFINITY),
                    );

                    // computes the size of the text with no width constraint
                    let max = text_pipeline.compute_size(
                        &sections,
                        &scaled_fonts,
                        text.alignment,
                        text.linebreak_behaviour,
                        Vec2::splat(f32::INFINITY),
                    );

                    let measure = TextMeasure {
                        min_content: min,
                        max_content: max,
                    };
                    calculated_size.measure = Box::new(measure);
                    text_queue.ids.push(entity);
                }
                Err(TextError::NoSuchFont) => {
                    new_queue.push(entity);
                }
                Err(e @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {e}.");
                }
            };
        }
    }
    *queued_text_ids = new_queue;

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
    mut queued_text: ResMut<TextQueue>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    text_settings: Res<TextSettings>,
    mut font_atlas_warning: ResMut<FontAtlasWarning>,
    ui_scale: Res<UiScale>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_queries: ParamSet<(
        Query<Entity, Or<(Changed<Text>, Changed<Node>)>>,
        Query<Entity, (With<Text>, With<Node>)>,
        Query<(&Node, &Text, &mut IntrinsicSize, &mut TextLayoutInfo)>,
    )>,
) {
    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let window_scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.);

    let scale_factor = ui_scale.scale * window_scale_factor;
    let inv_scale_factor = 1. / scale_factor;

    // Computes all text in the local queue
    let mut new_queue = Vec::new();
    let mut query = text_queries.p2();
    for entity in queued_text.ids.drain(..) {
        if let Ok((node, text, mut calculated_size, mut text_layout_info)) = query.get_mut(entity) {
            let node_size = Vec2::new(
                scale_value(node.size().x, scale_factor),
                scale_value(node.size().y, scale_factor),
            );
            match text_pipeline.queue_text(
                &fonts,
                &text.sections,
                scale_factor,
                text.alignment,
                text.linebreak_behaviour,
                node_size,
                &mut font_atlas_set_storage,
                &mut texture_atlases,
                &mut textures,
                text_settings.as_ref(),
                &mut font_atlas_warning,
                YAxisOrientation::TopToBottom,
            ) {
                Err(TextError::NoSuchFont) => {
                    // There was an error processing the text layout, let's add this entity to the
                    // queue for further processing
                    new_queue.push(entity);
                }
                Err(e @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {e}.");
                }
                Ok(info) => {
                    // calculated_size.size = Vec2::new(
                    //     scale_value(info.size.x, inv_scale_factor),
                    //     scale_value(info.size.y, inv_scale_factor),
                    // );
                    *text_layout_info = info;
                }
            }
        }
    }

    queued_text.ids = new_queue;
}
