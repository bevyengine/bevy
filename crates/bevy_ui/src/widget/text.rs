use crate::measurement::AvailableSpace;
use crate::{IntrinsicSize, Measure, Node, UiScale};
use bevy_asset::Assets;
use bevy_ecs::{
    entity::Entity,
    query::{Changed, Or, With},
    system::{Local, ParamSet, Query, Res, ResMut},
};
use bevy_math::Vec2;
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_text::{
    Font, FontAtlasSet, FontAtlasWarning, Text, TextError, TextLayoutInfo, TextPipeline,
    TextSettings, YAxisOrientation,
};
use bevy_window::{PrimaryWindow, Window};

fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}

#[derive(Clone)]
pub struct TextMeasure {
    pub min_content: Vec2,
    pub max_content: Vec2,
    pub ideal_height: f32,
}

impl Measure for TextMeasure {
    fn measure(
        &self,
        max_width: Option<f32>,
        max_height: Option<f32>,
        _: AvailableSpace,
        _: AvailableSpace,
    ) -> Vec2 {
        let mut size = Vec2::ZERO;
        match (max_width, max_height) {
            (None, None) => {
                // with no constraints
                // ask for maximum width space for text with no wrapping
                size = self.max_content;
            }
            (Some(width), None) => {
                size.x = width;
                size.y = self.ideal_height;
            }
            (None, Some(height)) => {
                size.y = height;
                size.x = self.max_content.x;
            }
            (Some(width), Some(height)) => {
                size.x = width;
                size.y = height;
            }
        }
        size.x = size.x.ceil();
        size.y = size.y.ceil();
        size
    }

    fn dyn_clone(&self) -> Box<dyn Measure> {
        Box::new(self.clone())
    }
}

/// Creates a `Measure` for text nodes that allows the UI to determine the appropriate amount of space
/// to provide for the text given the fonts, the text itself and the constraints of the layout.
pub fn measure_text_system(
    mut queued_text: Local<Vec<Entity>>,
    mut last_scale_factor: Local<f64>,
    fonts: Res<Assets<Font>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_queries: ParamSet<(
        Query<Entity, Changed<Text>>,
        Query<Entity, (With<Text>, With<Node>)>,
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
            if !queued_text.contains(&entity) {
                queued_text.push(entity);
            }
        }
    } else {
        // If the scale factor has changed, queue all text
        for entity in text_queries.p1().iter() {
            queued_text.push(entity);
        }
        *last_scale_factor = scale_factor;
    }

    if queued_text.is_empty() {
        return;
    }

    let mut new_queue = Vec::new();
    let mut query = text_queries.p2();
    for entity in queued_text.drain(..) {
        if let Ok((text, mut intrinsic_size)) = query.get_mut(entity) {
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
                        ideal_height: min.y,
                    };
                    intrinsic_size.min_content = min;
                    intrinsic_size.max_content = max;
                    intrinsic_size.ideal.y = min.y;
                    intrinsic_size.measure = Box::new(measure);
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
    *queued_text = new_queue;
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
    mut queued_text: Local<Vec<Entity>>,
    mut textures: ResMut<Assets<Image>>,
    mut last_scale_factor: Local<f64>,
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

    #[allow(clippy::float_cmp)]
    if *last_scale_factor == scale_factor {
        // Adds all entities where the text or the style has changed to the local queue
        for entity in text_queries.p0().iter() {
            if !queued_text.contains(&entity) {
                queued_text.push(entity);
            }
        }
    } else {
        // If the scale factor has changed, queue all text
        for entity in text_queries.p1().iter() {
            queued_text.push(entity);
        }
        *last_scale_factor = scale_factor;
    }

    let mut new_queue = Vec::new();
    let mut text_query = text_queries.p2();
    for entity in queued_text.drain(..) {
        if let Ok((node, text, mut intrinsic_size, mut text_layout_info)) =
            text_query.get_mut(entity)
        {
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
                    if info.size.y != intrinsic_size.ideal.y {
                        let measure = TextMeasure {
                            min_content: intrinsic_size.min_content,
                            max_content: intrinsic_size.max_content,
                            ideal_height: info.size.y,
                        };
                        intrinsic_size.ideal = info.size;
                        intrinsic_size.measure = Box::new(measure);
                    }
                    *text_layout_info = info;
                }
            }
        }
    }
    *queued_text = new_queue;
}
